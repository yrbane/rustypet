//! Le troupeau : un pet principal et ses enfants, chacun à sa cadence.
//! Voir `docs/reference/esheep-engine.md` §5.3 (childs).

use crate::geometry::World;
use crate::pet::{Pet, SpriteDraw, TickOutcome};
use pet_expr::PetRng;
use pet_format::PetDefinition;
use std::sync::Arc;

/// Nombre maximal d'enfants vivants en même temps : évite l'emballement
/// quand une animation fréquente déclare des enfants.
const MAX_CHILDREN: usize = 6;

/// Un acteur du troupeau et son échéance (ms restantes avant son tick).
struct Slot {
    pet: Pet,
    due_ms: i64,
}

/// Le pet principal et ses enfants, cadencés indépendamment : chaque acteur
/// respecte l'intervalle de sa propre animation, comme dans le moteur
/// d'origine où chaque sprite a son minuteur.
pub struct Flock {
    definition: Arc<PetDefinition>,
    tile: (i32, i32),
    /// Index 0 : le pet principal. Ensuite : les enfants.
    actors: Vec<Slot>,
}

impl Flock {
    pub fn new(definition: Arc<PetDefinition>, tile: (i32, i32), world: &World) -> Self {
        let pet = Pet::new(Arc::clone(&definition), tile, world);
        Self {
            definition,
            tile,
            actors: vec![Slot { pet, due_ms: 0 }],
        }
    }

    /// (Ré)apparition du pet principal ; les enfants sont congédiés.
    pub fn spawn(&mut self, world: &World, rng: &mut dyn PetRng) {
        self.actors.truncate(1);
        let main = &mut self.actors[0];
        main.pet.spawn(world, rng);
        main.due_ms = 0;
    }

    /// Fait passer `elapsed_ms` millisecondes : les acteurs arrivés à
    /// échéance avancent d'un pas, les enfants finis disparaissent, les
    /// enfants annoncés naissent.
    pub fn advance(&mut self, world: &World, rng: &mut dyn PetRng, elapsed_ms: i64) {
        let mut births: Vec<i32> = Vec::new();

        let mut index = 0;
        while index < self.actors.len() {
            let slot = &mut self.actors[index];
            slot.due_ms -= elapsed_ms;
            if slot.due_ms > 0 {
                index += 1;
                continue;
            }
            let outcome = slot.pet.tick(world, rng);
            slot.due_ms = i64::from(slot.pet.interval_ms());
            births.extend(slot.pet.pending_children());
            match outcome {
                TickOutcome::Close => {
                    self.actors.remove(index);
                }
                TickOutcome::Respawn => {
                    // Seul le principal réapparaît ; il repart seul.
                    self.spawn(world, rng);
                    index += 1;
                }
                TickOutcome::Continue => index += 1,
            }
        }

        for animation_id in births {
            self.hatch(animation_id, world, rng);
        }
    }

    /// Crée les enfants déclarés pour cette animation, dans la limite du
    /// plafond.
    fn hatch(&mut self, animation_id: i32, world: &World, rng: &mut dyn PetRng) {
        let childs: Vec<pet_format::Child> = self
            .definition
            .childs
            .iter()
            .filter(|c| c.animation_id == animation_id)
            .cloned()
            .collect();
        for child in childs {
            if self.actors.len() > MAX_CHILDREN {
                return;
            }
            let mut pet = Pet::new(Arc::clone(&self.definition), self.tile, world);
            pet.spawn_child(&child, world, rng);
            self.actors.push(Slot { pet, due_ms: 0 });
        }
    }

    /// Ce que chaque acteur affiche, le principal en premier.
    pub fn draws(&self) -> Vec<SpriteDraw> {
        self.actors.iter().map(|s| s.pet.draw()).collect()
    }

    /// Identifiants d'animation courants (diagnostic et simulateur).
    pub fn animation_ids(&self) -> Vec<i32> {
        self.actors.iter().map(|s| s.pet.animation_id()).collect()
    }

    /// Les sons tirés par le troupeau depuis le dernier appel (index dans
    /// `definition.sounds`).
    pub fn take_sounds(&mut self) -> Vec<usize> {
        self.actors
            .iter_mut()
            .filter_map(|s| s.pet.take_sound())
            .collect()
    }

    /// Délai avant la prochaine échéance, en millisecondes (au moins 1).
    pub fn next_wait_ms(&self) -> i64 {
        self.actors
            .iter()
            .map(|s| s.due_ms)
            .min()
            .unwrap_or(100)
            .max(1)
    }

    /// Le pet principal, pour le glisser-déposer et les tests.
    pub fn main_pet_mut(&mut self) -> &mut Pet {
        &mut self.actors[0].pet
    }

    /// Nombre d'acteurs vivants (principal compris).
    pub fn len(&self) -> usize {
        self.actors.len()
    }

    /// Jamais vide : le principal est toujours là.
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::SeededRng;
    use pet_format::parse_pet;

    /// Un pet dont la rencontre initiale (id 3, one-shot) engendre un
    /// enfant : l'enfant (id 5) trottine deux pas puis s'achève sans
    /// transition, donc se ferme. Le principal continue en marche (id 1).
    const XML_ENFANTS: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>100</x><y>200</y><next>3</next></spawn></spawns>
      <animations>
        <animation id="3">
          <name>meet</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame>
            <next probability="100">1</next></sequence>
        </animation>
        <animation id="1">
          <name>walk</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
          <border><next probability="100">1</next></border>
        </animation>
        <animation id="5">
          <name>child_walk</name>
          <start><x>-2</x><y>0</y><interval>50</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame></sequence>
        </animation>
      </animations>
      <childs>
        <child animationid="3"><x>300</x><y>200</y><next>5</next></child>
      </childs>
    </animations>"#;

    /// Variante : la marche, qui boucle sur elle-même, redéclare un enfant
    /// à chaque redémarrage — le plafond doit contenir la prolifération.
    const XML_PROLIFIQUE: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>100</x><y>200</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
          <border><next probability="100">1</next></border>
        </animation>
        <animation id="5">
          <name>child_walk</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="30" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">5</next></sequence>
        </animation>
      </animations>
      <childs>
        <child animationid="1"><x>300</x><y>200</y><next>5</next></child>
      </childs>
    </animations>"#;

    fn flock_from(xml: &str) -> (Flock, World, SeededRng) {
        let def = Arc::new(parse_pet(xml).expect("parsing"));
        let world = World::simple(800, 600);
        let flock = Flock::new(def, (32, 32), &world);
        (flock, world, SeededRng::new(7))
    }

    fn make_flock() -> (Flock, World, SeededRng) {
        flock_from(XML_ENFANTS)
    }

    #[test]
    fn le_principal_engendre_un_enfant_quand_l_animation_le_declare() {
        let (mut flock, world, mut rng) = make_flock();
        flock.spawn(&world, &mut rng);
        assert_eq!(flock.len(), 1, "au spawn, le principal est seul");

        // Le spawn a démarré l'animation 1 : l'enfant est en attente ; il
        // naît au premier passage de la boucle.
        flock.advance(&world, &mut rng, 0);
        assert_eq!(flock.len(), 2, "l'enfant déclaré doit naître");
        let draws = flock.draws();
        assert_eq!(
            (draws[1].x, draws[1].y),
            (300, 200),
            "l'enfant apparaît à la position déclarée"
        );
        assert_eq!(flock.animation_ids()[1], 5);
    }

    #[test]
    fn l_enfant_se_ferme_en_fin_de_chaine() {
        let (mut flock, world, mut rng) = make_flock();
        flock.spawn(&world, &mut rng);
        flock.advance(&world, &mut rng, 0);
        assert_eq!(flock.len(), 2);

        // L'animation 5 (2 frames, sans next) s'épuise en quelques pas de
        // 50 ms : l'enfant doit disparaître, le principal survivre.
        for _ in 0..10 {
            flock.advance(&world, &mut rng, 50);
        }
        assert_eq!(flock.len(), 1, "l'enfant fini doit se fermer");
    }

    #[test]
    fn le_plafond_d_enfants_est_respecte() {
        let (mut flock, world, mut rng) = flock_from(XML_PROLIFIQUE);
        flock.spawn(&world, &mut rng);
        // La marche boucle sur elle-même et redéclare un enfant à chaque
        // redémarrage : sans plafond, le troupeau exploserait.
        for _ in 0..200 {
            flock.advance(&world, &mut rng, 100);
        }
        assert!(
            flock.len() <= 1 + MAX_CHILDREN,
            "{} acteurs, plafond {} dépassé",
            flock.len(),
            1 + MAX_CHILDREN
        );
    }

    #[test]
    fn chaque_acteur_avance_a_sa_cadence() {
        let (mut flock, world, mut rng) = make_flock();
        flock.spawn(&world, &mut rng);
        flock.advance(&world, &mut rng, 0);
        let child_x0 = flock.draws()[1].x;

        // 100 ms : l'enfant (50 ms) doit avoir avancé deux fois (2 × -2 px),
        // en deux passages de 50 ms.
        flock.advance(&world, &mut rng, 50);
        flock.advance(&world, &mut rng, 50);
        let child_x1 = flock.draws()[1].x;
        assert_eq!(child_x0 - child_x1, 4, "deux pas d'enfant en 100 ms");

        // La prochaine échéance ne dépasse jamais l'intervalle le plus court.
        assert!(flock.next_wait_ms() <= 50);
    }
}
