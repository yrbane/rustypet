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

    // Boucle temps : avance le troupeau et émet l'état de tous les acteurs
    // à la cadence de la plus proche échéance d'animation.
    let mut elapsed: i64 = 0;
    loop {
        let (actors, wait) = {
            let mut engine = shared.lock().await;
            let frames = engine.advance(elapsed);
            (frames, engine.interval_ms())
        }; // mutex relâché avant de dormir

        let actors: Vec<(i32, i32, u32, bool, u32)> = actors
            .iter()
            .map(|f| (f.x, f.y, f.tile, f.flipped, f.opacity))
            .collect();
        let emitter = iface.signal_emitter();
        PetService::pet_state(emitter, actors).await?;

        elapsed = wait as i64;
        sleep(Duration::from_millis(wait)).await;
    }
}
