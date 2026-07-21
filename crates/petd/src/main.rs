//! Démon RustyPet : fait vivre un pet et publie son état sur D-Bus.

// Le vrai point d'entrée (tâche 3) consommera ces fonctions via D-Bus/tokio.
// En attendant, ce squelette n'appelle rien du module : on désactive le lint
// dead_code plutôt que d'inventer un appel factice.
#[allow(dead_code)]
mod cache;

fn main() {
    // Le vrai point d'entrée arrive en tâche 3.
    println!("petd — squelette");
}
