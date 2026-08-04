//! Le pet : état, physique et enchaînement des animations.
//! Voir `docs/reference/esheep-engine.md` §4.

use crate::anim_state::{interpolate, pick_frame, total_steps};
use crate::geometry::{Rect, World};
use crate::transitions::{pick_next, pick_spawn};
use pet_expr::{EvalContext, PetRng};
use pet_format::{OnlyFlags, PetDefinition};
use std::sync::Arc;

/// Ce que le moteur demande d'afficher pour ce pet, à cet instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteDraw {
    /// Index de tuile dans le spritesheet.
    pub frame: i32,
    pub x: i32,
    pub y: i32,
    pub opacity: f64,
    /// Le sprite doit être affiché en miroir horizontal.
    pub flipped: bool,
}

/// Ce qu'il faut faire du pet après un tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// Le pet continue de vivre.
    Continue,
    /// Aucune animation suivante : le pet principal réapparaît.
    Respawn,
    /// Aucune animation suivante : un enfant disparaît.
    Close,
}

/// Un pet vivant.
pub struct Pet {
    definition: Arc<PetDefinition>,
    /// Dimensions d'une tuile, en pixels.
    tile_w: i32,
    tile_h: i32,
    ctx: EvalContext,

    animation_id: i32,
    step: i32,
    total: i32,

    position_x: i32,
    position_y: i32,
    offset_y: i32,
    opacity: f64,
    interval: i32,
    frame: i32,

    /// Faux quand le pet se déplace vers la droite ; l'axe X est alors nié.
    moving_left: bool,
    dragging: bool,
    drag_x: i32,
    drag_y: i32,

    /// Fenêtre dont le pet arpente le toit (§4.3, §4.5). `None` = au sol
    /// ou en l'air ; les bords suivis sont alors ceux de l'écran.
    on_window: Option<Rect>,

    /// Vrai pour un enfant : il se ferme au lieu de réapparaître.
    is_child: bool,
    /// Animations dont les enfants restent à créer.
    pending_children: Vec<i32>,
    /// Animation courante, expressions déjà évaluées.
    cache: Option<pet_format::Animation>,
}

impl Pet {
    /// Crée un pet à partir de sa définition et de la taille de ses tuiles.
    pub fn new(definition: Arc<PetDefinition>, tile: (i32, i32), world: &World) -> Self {
        let (tile_w, tile_h) = tile;
        let ctx = EvalContext {
            screen_w: world.bounds.w,
            screen_h: world.bounds.h,
            area_w: world.area.w,
            // areaH est le bord bas de la zone de travail (§3.1).
            area_h: world.area.bottom(),
            image_w: tile_w,
            image_h: tile_h,
            image_x: -1,
            image_y: -1,
            rand_spawn: 50,
            scale: 1,
        };

        Self {
            definition,
            tile_w,
            tile_h,
            ctx,
            animation_id: -1,
            step: -1,
            total: 0,
            position_x: 0,
            position_y: 0,
            offset_y: 0,
            opacity: 1.0,
            interval: 100,
            frame: 0,
            moving_left: true,
            dragging: false,
            drag_x: 0,
            drag_y: 0,
            on_window: None,
            is_child: false,
            pending_children: Vec::new(),
            cache: None,
        }
    }

    /// Marque ce pet comme enfant : il se ferme au lieu de réapparaître.
    pub fn set_child(&mut self, is_child: bool) {
        self.is_child = is_child;
    }

    /// Identifiant de l'animation en cours.
    pub fn animation_id(&self) -> i32 {
        self.animation_id
    }

    /// Position courante, coin haut-gauche.
    pub fn position(&self) -> (i32, i32) {
        (self.position_x, self.position_y)
    }

    /// Force la position. Réservé aux tests et au placement des enfants.
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position_x = x;
        self.position_y = y;
    }

    /// Force l'orientation. Réservé aux tests.
    pub fn set_flipped(&mut self, flipped: bool) {
        self.moving_left = !flipped;
    }

    /// Durée à attendre avant le prochain tick, en millisecondes.
    pub fn interval_ms(&self) -> i32 {
        self.interval.max(1)
    }

    /// Ce qu'il faut afficher maintenant.
    pub fn draw(&self) -> SpriteDraw {
        SpriteDraw {
            frame: self.frame,
            x: self.position_x,
            y: self.position_y + self.offset_y,
            opacity: self.opacity,
            flipped: !self.moving_left,
        }
    }

    /// Récupère et vide la liste des enfants à créer.
    pub fn pending_children(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.pending_children)
    }

    /// Fait apparaître le pet à un point d'apparition tiré au sort.
    pub fn spawn(&mut self, world: &World, rng: &mut dyn PetRng) {
        // randS est figé pour toute la durée de vie du pet (§3.1).
        self.ctx.rand_spawn = rng.gen_range_i32(10, 90);
        self.refresh_context(world);

        let spawns = &self.definition.spawns;
        let spawn_option = pick_spawn(spawns, rng).and_then(|index| spawns.get(index));

        let (x, y, next) = if let Some(spawn) = spawn_option {
            let mut spawn = spawn.clone();
            spawn.x.update(&self.ctx, rng, false);
            spawn.y.update(&self.ctx, rng, false);
            (spawn.x.get(), spawn.y.get(), spawn.next)
        } else {
            // Spawn de secours quand le XML n'en définit aucun (§5.1).
            let first = self
                .definition
                .animations
                .first()
                .map(|a| a.id)
                .unwrap_or(1);
            (0, 0, first)
        };

        // Miroir horizontal de la position d'apparition si le pet est retourné.
        self.position_x = if self.moving_left {
            world.bounds.x + x
        } else {
            world.bounds.x - (x - world.bounds.w) - self.tile_w
        };
        self.position_y = world.bounds.y + y;
        self.offset_y = 0;
        self.opacity = 1.0;
        self.on_window = None;

        self.set_animation(next, world, rng);
    }

    /// Fait apparaître ce pet comme enfant : position déclarée par le
    /// `<child>`, fermeture (au lieu du respawn) en fin de chaîne (§5.3).
    pub fn spawn_child(&mut self, child: &pet_format::Child, world: &World, rng: &mut dyn PetRng) {
        self.is_child = true;
        self.ctx.rand_spawn = rng.gen_range_i32(10, 90);
        self.refresh_context(world);

        let mut child = child.clone();
        child.x.update(&self.ctx, rng, false);
        child.y.update(&self.ctx, rng, false);
        self.position_x = world.bounds.x + child.x.get();
        self.position_y = world.bounds.y + child.y.get();
        self.offset_y = 0;
        self.opacity = 1.0;
        self.on_window = None;

        self.set_animation(child.next, world, rng);
    }

    /// Démarre une animation. Le premier tick portera le pas 0.
    fn set_animation(&mut self, id: i32, world: &World, rng: &mut dyn PetRng) {
        self.refresh_context(world);

        let Some(animation) = self.definition.animation(id) else {
            return;
        };
        let mut animation = animation.clone();

        // Réévalue les expressions dynamiques au démarrage (§3.4).
        animation.sequence.repeat.update(&self.ctx, rng, false);
        animation.start.x.update(&self.ctx, rng, false);
        animation.start.y.update(&self.ctx, rng, false);
        animation.start.interval.update(&self.ctx, rng, false);
        if let Some(end) = animation.end.as_mut() {
            end.x.update(&self.ctx, rng, false);
            end.y.update(&self.ctx, rng, false);
            end.interval.update(&self.ctx, rng, false);
        }

        self.total = total_steps(&animation.sequence);
        self.animation_id = id;
        self.step = 0;
        self.interval = animation.start.interval.get().max(1);
        self.frame = pick_frame(&animation.sequence, 0);

        // Cette animation crée-t-elle des enfants ?
        if self.definition.childs.iter().any(|c| c.animation_id == id) {
            self.pending_children.push(id);
        }

        self.cache = Some(animation);
    }

    /// Met à jour le contexte d'évaluation avec la géométrie courante.
    fn refresh_context(&mut self, world: &World) {
        self.ctx.screen_w = world.bounds.w;
        self.ctx.screen_h = world.bounds.h;
        self.ctx.area_w = world.area.w;
        self.ctx.area_h = world.area.bottom();
        self.ctx.image_w = self.tile_w;
        self.ctx.image_h = self.tile_h;
    }

    /// Le pet est attrapé à la souris.
    pub fn begin_drag(&mut self, world: &World, rng: &mut dyn PetRng) {
        self.dragging = true;
        if let Some(id) = self.definition.animation_id_by_name("drag") {
            self.set_animation(id, world, rng);
        }
    }

    /// Le curseur a bougé pendant le glisser. La position suit sans attendre
    /// le prochain tick : aucune latence visuelle.
    pub fn drag_to(&mut self, x: i32, y: i32) {
        self.drag_x = x;
        self.drag_y = y;
        if self.dragging {
            self.position_x = x - self.tile_w / 2;
            self.position_y = y - 2;
        }
    }

    /// Le pet est relâché : il tombe.
    pub fn end_drag(&mut self, world: &World, rng: &mut dyn PetRng) {
        self.dragging = false;
        if let Some(id) = self.definition.animation_id_by_name("fall") {
            self.set_animation(id, world, rng);
        }
    }

    /// Avance d'un pas. Voir l'ordre des opérations en §4.2.
    pub fn tick(&mut self, world: &World, rng: &mut dyn PetRng) -> TickOutcome {
        let Some(animation) = self.cache.clone() else {
            return TickOutcome::Continue;
        };

        self.frame = pick_frame(&animation.sequence, self.step);

        // 1. Pendant le glisser, la position suit le curseur, sans physique.
        if self.dragging {
            self.position_x = self.drag_x - self.tile_w / 2;
            self.position_y = self.drag_y - 2;
            self.step += 1;
            return TickOutcome::Continue;
        }

        // 2. Interpolation des valeurs du pas courant.
        let values = interpolate(&animation, self.step, self.total);
        self.interval = values.interval.max(1);
        self.opacity = values.opacity;
        self.offset_y = values.offset_y;

        let mut dx = values.x;
        let mut dy = values.y;

        // 3. L'axe X est nié quand le pet est retourné.
        if !self.moving_left {
            dx = -dx;
        }

        // 4. Détections de bord, dans l'ordre du moteur d'origine. Un bord
        // franchi sans transition éligible signifie « hors écran » : la spec
        // impose alors le respawn du pet principal (fermeture pour un enfant),
        // sinon il continuerait à l'infini dans le vide.
        let mut next_animation: Option<i32> = None;

        if let Some(rect) = self.on_window {
            // Sur une fenêtre, les bords suivis sont ceux de la fenêtre
            // (§4.3). Sans transition éligible, le pet quitte simplement le
            // toit et la gravité fera le reste.
            if dx < 0 && self.position_x + dx < rect.x {
                match pick_next(&animation.border, OnlyFlags::WINDOW, rng) {
                    Some(id) => {
                        self.position_x = rect.x;
                        dx = 0;
                        next_animation = Some(id);
                    }
                    None => self.on_window = None,
                }
            } else if dx > 0 && self.position_x + dx + self.tile_w > rect.right() {
                match pick_next(&animation.border, OnlyFlags::WINDOW, rng) {
                    Some(id) => {
                        self.position_x = rect.right() - self.tile_w;
                        dx = 0;
                        next_animation = Some(id);
                    }
                    None => self.on_window = None,
                }
            }
        } else if dx < 0 && self.position_x + dx < world.area.x {
            match pick_next(&animation.border, OnlyFlags::VERTICAL, rng) {
                Some(id) => {
                    self.position_x = world.area.x;
                    dx = 0;
                    next_animation = Some(id);
                }
                None => return self.offscreen_outcome(),
            }
        } else if dx > 0 && self.position_x + dx + self.tile_w > world.area.right() {
            match pick_next(&animation.border, OnlyFlags::VERTICAL, rng) {
                Some(id) => {
                    self.position_x = world.area.right() - self.tile_w;
                    dx = 0;
                    next_animation = Some(id);
                }
                None => return self.offscreen_outcome(),
            }
        }

        // Atterrissage sur le toit d'une fenêtre pendant une descente (§4.5).
        // Sans transition `only="window"` éligible, le pet traverse.
        if next_animation.is_none()
            && dy > 0
            && let Some(rect) = self.landing_window(world, dy)
            && let Some(id) = pick_next(&animation.border, OnlyFlags::WINDOW, rng)
        {
            self.position_y = rect.y - self.tile_h;
            self.offset_y = 0;
            dy = 0;
            self.on_window = Some(rect);
            next_animation = Some(id);
        }

        let floor = world.area.bottom() - self.tile_h;
        if next_animation.is_none() {
            if dy > 0 && self.position_y + dy > floor {
                match pick_next(&animation.border, OnlyFlags::TASKBAR, rng) {
                    Some(id) => {
                        self.position_y = floor;
                        self.offset_y = 0;
                        dy = 0;
                        next_animation = Some(id);
                    }
                    None => return self.offscreen_outcome(),
                }
            } else if dy < 0 && self.position_y + dy < world.area.y {
                match pick_next(&animation.border, OnlyFlags::HORIZONTAL, rng) {
                    Some(id) => {
                        self.position_y = world.area.y;
                        dy = 0;
                        next_animation = Some(id);
                    }
                    None => return self.offscreen_outcome(),
                }
            }
        }

        // 5. Gravité : le pet tombe s'il n'est porté ni par le sol ni par une
        // fenêtre (§4.4).
        if next_animation.is_none() && animation.has_gravity() {
            if self.on_window.is_some() {
                // La fenêtre est-elle toujours là, toujours sous les pieds ?
                // Elle a pu bouger : on suit son rectangle actuel.
                match self.window_under_feet(world) {
                    Some(rect) => self.on_window = Some(rect),
                    None => {
                        self.on_window = None;
                        if let Some(id) = pick_next(&animation.gravity, OnlyFlags::WINDOW, rng) {
                            next_animation = Some(id);
                        }
                    }
                }
            } else if self.position_y + dy < floor {
                // Tolérance de 3 px : on colle au sol plutôt que de déclencher une chute.
                if self.position_y + dy + 3 >= floor {
                    dy = floor - self.position_y;
                } else if let Some(id) = pick_next(&animation.gravity, OnlyFlags::NONE, rng) {
                    next_animation = Some(id);
                }
            }
        }

        // 6. Fin de séquence.
        let mut outcome = TickOutcome::Continue;
        if next_animation.is_none() && self.step >= self.total {
            if animation.sequence.action.as_deref() == Some("flip") {
                self.moving_left = !self.moving_left;
            }
            match pick_next(&animation.sequence.next, OnlyFlags::NONE, rng) {
                Some(id) => next_animation = Some(id),
                None => {
                    outcome = if self.is_child {
                        TickOutcome::Close
                    } else {
                        TickOutcome::Respawn
                    };
                }
            }
        }

        // 7. Application du déplacement.
        self.position_x += dx;
        self.position_y += dy;
        self.step += 1;

        if let Some(id) = next_animation {
            self.set_animation(id, world, rng);
            // Le moteur d'origine affiche la première frame quasi immédiatement.
            self.interval = 1;
        }

        outcome
    }

    /// La fenêtre que le pet, en descente, atteindrait pendant ce pas (§4.5) :
    /// pieds au-dessus du toit avant le pas, dessous après, avec une tolérance
    /// latérale d'une demi-tuile, et pas trop haut sur l'écran.
    fn landing_window(&self, world: &World, dy: i32) -> Option<Rect> {
        let feet = self.position_y + self.tile_h;
        world
            .windows
            .iter()
            .find(|w| {
                feet < w.y
                    && feet + dy >= w.y
                    && self.position_x >= w.x - self.tile_w / 2
                    && self.position_x + self.tile_w <= w.right() + self.tile_w / 2
                    && self.position_y > world.area.y + 20
            })
            .copied()
    }

    /// La fenêtre encore présente sous les pieds du pet, s'il y en a une.
    fn window_under_feet(&self, world: &World) -> Option<Rect> {
        let feet = self.position_y + self.tile_h;
        world
            .windows
            .iter()
            .find(|w| {
                (feet - w.y).abs() <= 3
                    && self.position_x + self.tile_w > w.x - self.tile_w / 2
                    && self.position_x < w.right() + self.tile_w / 2
            })
            .copied()
    }

    /// Issue d'un pet qui franchit un bord sans transition éligible.
    fn offscreen_outcome(&self) -> TickOutcome {
        if self.is_child {
            TickOutcome::Close
        } else {
            TickOutcome::Respawn
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::SeededRng;
    use pet_format::parse_pet;
    use std::sync::Arc;

    /// Un pet minimal : marche vers la droite, rebondit sur les bords.
    const XML: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>100</x><y>200</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>5</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
          <border><next probability="100">2</next></border>
        </animation>
        <animation id="2">
          <name>turn</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><action>flip</action>
            <next probability="100">1</next></sequence>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    fn make_pet() -> (Pet, World, SeededRng) {
        let def = Arc::new(parse_pet(XML).expect("parsing"));
        let world = World::simple(800, 600);
        let pet = Pet::new(def, (32, 32), &world);
        (pet, world, SeededRng::new(7))
    }

    #[test]
    fn le_spawn_positionne_le_pet() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        assert_eq!(pet.position(), (100, 200));
        assert_eq!(pet.draw().frame, 0);
    }

    #[test]
    fn le_pet_avance_a_chaque_tick() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        let (x0, _) = pet.position();
        pet.tick(&world, &mut rng);
        let (x1, _) = pet.position();
        assert_eq!(x1 - x0, 5, "le pet doit avancer de 5 px par pas");
    }

    #[test]
    fn le_pet_s_arrete_au_bord_droit() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        pet.set_position(790, 200); // proche du bord droit (800 - 32)
        for _ in 0..10 {
            pet.tick(&world, &mut rng);
        }
        let (x, _) = pet.position();
        assert!(
            x <= world.area.right() - 32,
            "le pet est sorti à droite : x = {x}"
        );
    }

    #[test]
    fn le_pet_ne_sort_pas_par_la_gauche() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        // L'animation par défaut de ce pet de test avance vers la droite
        // (start.x = 5, cf. XML plus haut) ; `set_flipped(true)` met donc
        // `moving_left` à `false`, ce qui nie ce déplacement et fait
        // effectivement avancer le pet vers la gauche (vérifié : la position
        // décroît de 5 px par tick). Sans ce retournement, le test serait
        // trivialement vert puisque le pet s'éloignerait du bord gauche.
        pet.set_flipped(true);
        pet.set_position(2, 200);
        for _ in 0..10 {
            pet.tick(&world, &mut rng);
        }
        let (x, _) = pet.position();
        assert!(x >= world.area.x, "le pet est sorti à gauche : x = {x}");
    }

    #[test]
    fn le_drag_suspend_la_physique() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        pet.begin_drag(&world, &mut rng);
        pet.drag_to(400, 300);
        pet.tick(&world, &mut rng);
        // La position reste celle du curseur, à l'ancrage près.
        let (x, y) = pet.position();
        assert_eq!(x, 400 - 16); // curseur - largeur/2
        assert_eq!(y, 300 - 2);
    }

    #[test]
    fn la_simulation_est_deterministe_a_graine_egale() {
        let trace = |seed: u64| {
            let def = Arc::new(parse_pet(XML).expect("parsing"));
            let world = World::simple(800, 600);
            let mut pet = Pet::new(def, (32, 32), &world);
            let mut rng = SeededRng::new(seed);
            pet.spawn(&world, &mut rng);
            (0..200)
                .map(|_| {
                    pet.tick(&world, &mut rng);
                    let d = pet.draw();
                    (d.frame, d.x, d.y, d.flipped)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(trace(1234), trace(1234));
    }

    /// Même pet mais sans transition `<border>` : le cas des animations de
    /// marche du neko réel, qui sortaient de l'écran à l'infini.
    const XML_SANS_BORDER: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>100</x><y>200</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>5</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    #[test]
    fn sans_transition_de_bord_le_pet_respawne_au_lieu_de_sortir() {
        let def = Arc::new(parse_pet(XML_SANS_BORDER).expect("parsing"));
        let world = World::simple(800, 600);
        let mut pet = Pet::new(def, (32, 32), &world);
        let mut rng = SeededRng::new(7);
        pet.spawn(&world, &mut rng);
        // Marche vers la gauche (cf. le_pet_ne_sort_pas_par_la_gauche).
        pet.set_flipped(true);
        pet.set_position(2, 200);

        let mut respawned = false;
        for step in 0..50 {
            if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
                respawned = true;
                pet.spawn(&world, &mut rng);
            }
            let (x, _) = pet.position();
            assert!(x >= world.area.x, "sorti à gauche au pas {step} : x = {x}");
        }
        assert!(
            respawned,
            "le bord sans transition doit provoquer un respawn"
        );
    }

    /// Un pet « façon mouton » : il marche (avec gravité) et sait tomber.
    /// La chute (`fall`) atterrit sur une fenêtre (`only="window"`) ou sur la
    /// barre des tâches ; la marche ne connaît que les bords d'écran
    /// (`only="vertical"`), donc au bord d'une fenêtre elle quitte le toit.
    const XML_FENETRES: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>150</x><y>100</y><next>2</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>5</x><y>0</y><interval>100</interval></start>
          <sequence repeat="10" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
          <border><next probability="100" only="vertical">1</next></border>
          <gravity><next probability="100" only="window">2</next>
            <next probability="100">2</next></gravity>
        </animation>
        <animation id="2">
          <name>fall</name>
          <start><x>0</x><y>8</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">2</next></sequence>
          <border><next probability="100" only="window">1</next>
            <next probability="100" only="taskbar">1</next></border>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    /// Monde 800×600 avec une fenêtre dont le toit est à y = 300.
    fn make_pet_fenetres() -> (Pet, World, SeededRng) {
        let def = Arc::new(parse_pet(XML_FENETRES).expect("parsing"));
        let mut world = World::simple(800, 600);
        world
            .windows
            .push(crate::geometry::Rect::new(100, 300, 300, 200));
        let pet = Pet::new(def, (32, 32), &world);
        (pet, world, SeededRng::new(7))
    }

    #[test]
    fn le_pet_atterrit_sur_le_toit_d_une_fenetre() {
        let (mut pet, world, mut rng) = make_pet_fenetres();
        pet.spawn(&world, &mut rng); // (150, 100), en chute
        for _ in 0..40 {
            pet.tick(&world, &mut rng);
        }
        let (_, y) = pet.position();
        assert_eq!(
            y,
            300 - 32,
            "le pet doit se poser sur le toit de la fenêtre, pas au sol"
        );
    }

    #[test]
    fn le_pet_tombe_du_bord_de_la_fenetre_et_atteint_le_sol() {
        let (mut pet, world, mut rng) = make_pet_fenetres();
        pet.spawn(&world, &mut rng);
        for _ in 0..200 {
            pet.tick(&world, &mut rng);
        }
        // Largement le temps d'atterrir, de traverser le toit à 5 px/pas,
        // puis de chuter jusqu'au sol.
        let (_, y) = pet.position();
        assert_eq!(
            y,
            600 - 32,
            "après le bord du toit, le pet doit finir au sol"
        );
    }

    #[test]
    fn le_pet_chute_quand_la_fenetre_disparait_sous_lui() {
        let (mut pet, mut world, mut rng) = make_pet_fenetres();
        pet.spawn(&world, &mut rng);
        for _ in 0..40 {
            pet.tick(&world, &mut rng);
        }
        assert_eq!(pet.position().1, 268, "prérequis : le pet est sur le toit");
        world.windows.clear();
        for _ in 0..80 {
            pet.tick(&world, &mut rng);
        }
        assert_eq!(
            pet.position().1,
            600 - 32,
            "fenêtre fermée : le pet doit retomber au sol"
        );
    }

    #[test]
    fn le_pet_reste_dans_l_ecran_sur_une_longue_simulation() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        for step in 0..2000 {
            pet.tick(&world, &mut rng);
            let (x, y) = pet.position();
            // Clampage strict : le pet reste dans [area.x, area.right() - tile_w]
            // horizontalement et [area.y, area.bottom() - tile_h] verticalement.
            assert!(
                x >= world.area.x && x <= world.area.right() - 32,
                "sorti horizontalement au pas {step} : x = {x}"
            );
            assert!(
                y >= world.area.y && y <= world.area.bottom() - 32,
                "sorti verticalement au pas {step} : y = {y}"
            );
        }
    }
}
