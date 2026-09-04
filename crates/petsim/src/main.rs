//! Simulateur headless : fait vivre un pet sans écran et dumpe sa trace.
//!
//! Sert de backend « null » et d'outil de non-régression : la trace est
//! reproductible à graine égale.

use clap::Parser;
use pet_engine::{Flock, Rect, SeededRng, World};
use pet_format::{decode_sheet, parse_pet};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "petsim", about = "Simule un pet eSheep sans affichage")]
struct Args {
    /// Chemin du fichier animations.xml
    xml: String,
    /// Nombre de pas à simuler
    #[arg(long, default_value_t = 100)]
    ticks: u32,
    /// Graine du générateur aléatoire
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Largeur de l'écran simulé
    #[arg(long, default_value_t = 1920)]
    width: i32,
    /// Hauteur de l'écran simulé
    #[arg(long, default_value_t = 1080)]
    height: i32,
    /// Fenêtre simulée « x,y,largeur,hauteur » (répétable)
    #[arg(long = "window", value_parser = parse_rect)]
    windows: Vec<Rect>,
}

/// Lit un rectangle « x,y,l,h » depuis la ligne de commande.
fn parse_rect(s: &str) -> Result<Rect, String> {
    let parts: Vec<i32> = s
        .split(',')
        .map(|p| p.trim().parse().map_err(|e| format!("{e}")))
        .collect::<Result<_, _>>()?;
    match parts[..] {
        [x, y, w, h] => Ok(Rect::new(x, y, w, h)),
        _ => Err(format!("attendu « x,y,largeur,hauteur », reçu « {s} »")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let xml = std::fs::read_to_string(&args.xml)?;
    let definition = Arc::new(parse_pet(&xml)?);
    let sheet = decode_sheet(&definition.image)?;

    println!(
        "# pet={} tuiles={}x{} taille={}x{} animations={}",
        definition.header.petname,
        sheet.tiles_x,
        sheet.tiles_y,
        sheet.tile_w,
        sheet.tile_h,
        definition.animations.len()
    );

    let mut world = World::simple(args.width, args.height);
    world.windows = args.windows.clone();
    let mut rng = SeededRng::new(args.seed);
    let mut flock = Flock::new(
        definition,
        (sheet.tile_w as i32, sheet.tile_h as i32),
        &world,
    );
    flock.spawn(&world, &mut rng);

    // Le temps simulé avance d'échéance en échéance, comme le démon.
    let mut elapsed = 0;
    for tick in 0..args.ticks {
        flock.advance(&world, &mut rng, elapsed);
        let draws = flock.draws();
        let ids = flock.animation_ids();
        let main = draws[0];
        print!(
            "{tick:04} anim={:3} frame={:3} x={:5} y={:5} opacite={:.2} miroir={} attente={}ms",
            ids[0],
            main.frame,
            main.x,
            main.y,
            main.opacity,
            main.flipped,
            flock.next_wait_ms()
        );
        // Les enfants du troupeau, à la suite sur la même ligne.
        for (draw, id) in draws.iter().zip(ids.iter()).skip(1) {
            print!(" | enfant anim={} x={} y={}", id, draw.x, draw.y);
        }
        println!();
        elapsed = flock.next_wait_ms();
    }

    Ok(())
}
