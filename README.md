# rustypet

Moteur d'animaux de bureau. Lecture et interprétation du format eSheep.

## Développement

Lancer la suite de tests :

```bash
cargo test
```

Valider le corpus complet du dépôt amont (22 pets) :

```bash
RUSTYPET_CORPUS=~/Dev/desktopPet/Pets cargo test -p pet-format -- --ignored
```

Linter et formateur :

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```
