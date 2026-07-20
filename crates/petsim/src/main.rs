//! Simulateur headless : fait vivre un pet sans écran et dumpe sa trace.
//!
//! Sert de backend « null » et d'outil de non-régression : la trace est
//! reproductible à graine égale.

use clap::Parser;
use pet_engine::{Pet, TickOutcome, World};
use pet_expr::SeededRng;
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

    let world = World::simple(args.width, args.height);
    let mut rng = SeededRng::new(args.seed);
    let mut pet = Pet::new(
        definition,
        (sheet.tile_w as i32, sheet.tile_h as i32),
        &world,
    );
    pet.spawn(&world, &mut rng);

    let mut respawns = 0;
    for tick in 0..args.ticks {
        let outcome = pet.tick(&world, &mut rng);
        let draw = pet.draw();
        println!(
            "{tick:04} frame={:3} x={:5} y={:5} opacite={:.2} miroir={} attente={}ms",
            draw.frame,
            draw.x,
            draw.y,
            draw.opacity,
            draw.flipped,
            pet.interval_ms()
        );
        if outcome == TickOutcome::Respawn {
            respawns += 1;
            pet.spawn(&world, &mut rng);
        }
    }

    println!("# réapparitions={respawns}");
    Ok(())
}
