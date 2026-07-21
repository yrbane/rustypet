//! Démon RustyPet : fait vivre un pet et publie son état sur D-Bus.

mod cache;
mod engine;
mod service;

use clap::Parser;
use engine::Engine;
use service::PetService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

#[derive(Parser)]
#[command(name = "petd", about = "Démon d'animaux de bureau RustyPet")]
struct Args {
    /// Chemin du fichier animations.xml du pet à afficher.
    xml: String,
    /// Graine du générateur aléatoire.
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let engine = Engine::load(&args.xml, args.seed)?;
    let shared = Arc::new(Mutex::new(engine));

    let service = PetService {
        engine: Arc::clone(&shared),
    };
    let conn = zbus::connection::Builder::session()?
        .name("dev.yrbane.RustyPet")?
        .serve_at("/dev/yrbane/RustyPet", service)?
        .build()
        .await?;

    // Référence vers l'interface, pour émettre les signaux.
    let iface = conn
        .object_server()
        .interface::<_, PetService>("/dev/yrbane/RustyPet")
        .await?;

    // Boucle temps : avance le pet et émet son état à la cadence des animations.
    loop {
        let (frame, wait) = {
            let mut engine = shared.lock().await;
            let frame = engine.advance();
            (frame, engine.interval_ms())
        }; // mutex relâché avant de dormir

        let emitter = iface.signal_emitter();
        PetService::pet_state(
            emitter,
            frame.x,
            frame.y,
            frame.tile,
            frame.flipped,
            frame.opacity,
        )
        .await?;

        sleep(Duration::from_millis(wait)).await;
    }
}
