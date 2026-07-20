//! Lecture tolérante d'un `animations.xml`.
//!
//! Le schéma utilise `xsd:all` : l'ordre des sous-éléments est libre. Le
//! parseur construit donc un arbre générique puis lit les champs par nom.

use crate::FormatError;
use crate::model::*;
use pet_expr::PetValue;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;

/// Nœud générique : texte, attributs, enfants par nom.
#[derive(Debug, Default, Clone)]
pub(crate) struct Node {
    pub text: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<(String, Node)>,
}

impl Node {
    /// Premier enfant portant ce nom.
    fn child(&self, name: &str) -> Option<&Node> {
        self.children
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c)
    }

    /// Tous les enfants portant ce nom.
    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children
            .iter()
            .filter(move |(n, _)| n == name)
            .map(|(_, c)| c)
    }

    /// Texte d'un enfant, ou la valeur par défaut.
    fn text_of(&self, name: &str, default: &str) -> String {
        self.child(name)
            .map(|c| c.text.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string())
    }

    /// Entier d'un enfant, ou la valeur par défaut.
    fn int_of(&self, name: &str, default: i32) -> i32 {
        self.text_of(name, "").parse().unwrap_or(default)
    }

    /// Attribut entier, ou la valeur par défaut.
    fn attr_int(&self, name: &str, default: i32) -> i32 {
        self.attrs
            .get(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

/// Construit l'arbre générique à partir du XML.
///
/// Les préfixes de namespace sont retirés : les pets déclarent un namespace
/// par défaut que l'on ignore volontairement. Aucun panic n'est possible ici :
/// une pile vide inattendue dégrade silencieusement plutôt que de paniquer.
pub(crate) fn build_tree(xml: &str) -> Result<Node, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<(String, Node)> = vec![(String::from("#root"), Node::default())];

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                let mut node = Node::default();
                for attr in e.attributes().flatten() {
                    let key = local_name(attr.key.as_ref());
                    let value = attr.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, value);
                }
                stack.push((name, node));
            }
            Ok(Event::Empty(e)) => {
                let name = local_name(e.name().as_ref());
                let mut node = Node::default();
                for attr in e.attributes().flatten() {
                    let key = local_name(attr.key.as_ref());
                    let value = attr.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, value);
                }
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push((name, node));
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    // `stack.len() > 1` garantit que `pop` réussit, mais on
                    // reste défensif plutôt que d'appeler `expect`.
                    if let Some((name, node)) = stack.pop()
                        && let Some((_, parent)) = stack.last_mut()
                    {
                        parent.children.push((name, node));
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if let Some((_, node)) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some((_, node)) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(FormatError::Xml(e.to_string())),
        }
    }

    // Un flux bien formé laisse toujours la racine `#root` en fond de pile ;
    // un flux tronqué peut la faire manquer, d'où le repli sur un nœud vide
    // plutôt qu'un panic.
    let root = stack.pop().map(|(_, node)| node).unwrap_or_default();
    Ok(root)
}

/// Retire le préfixe de namespace d'un nom qualifié.
fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => s.to_string(),
    }
}

/// Lit un `<next>` : contenu = identifiant, attributs `probability` et `only`.
fn parse_next(node: &Node) -> NextAnimation {
    NextAnimation {
        id: node.text.trim().parse().unwrap_or(-1),
        probability: node.attr_int("probability", 100),
        only: OnlyFlags::parse(node.attrs.get("only").map(String::as_str).unwrap_or("")),
    }
}

/// Lit un groupe `step` (`<start>` ou `<end>`).
fn parse_movement(node: &Node) -> Movement {
    Movement {
        x: PetValue::new(node.text_of("x", "0")),
        y: PetValue::new(node.text_of("y", "0")),
        interval: PetValue::new(node.text_of("interval", "100")),
        offset_y: node.int_of("offsety", 0),
        opacity: node.text_of("opacity", "1.0").parse().unwrap_or(1.0),
    }
}

/// Lit une `<sequence>` : `frame`, `action` et `next` sont mêlés librement.
fn parse_sequence(node: &Node) -> Sequence {
    let mut frames = Vec::new();
    let mut action = None;
    let mut next = Vec::new();

    for (name, child) in &node.children {
        match name.as_str() {
            "frame" => {
                if let Ok(v) = child.text.trim().parse::<i32>() {
                    frames.push(v);
                }
            }
            "action" => {
                let value = child.text.trim();
                if !value.is_empty() {
                    action = Some(value.to_string());
                }
            }
            "next" => next.push(parse_next(child)),
            _ => {}
        }
    }

    Sequence {
        repeat: PetValue::new(
            node.attrs
                .get("repeat")
                .cloned()
                .unwrap_or_else(|| "0".to_string()),
        ),
        repeat_from: node.attr_int("repeatfrom", 0),
        frames,
        action,
        next,
    }
}

/// Lit tous les `<next>` d'un conteneur `<border>` ou `<gravity>`.
fn parse_next_list(node: Option<&Node>) -> Vec<NextAnimation> {
    node.map(|n| n.children_named("next").map(parse_next).collect())
        .unwrap_or_default()
}

/// Lit un `animations.xml` complet.
pub fn parse_pet(xml: &str) -> Result<PetDefinition, FormatError> {
    let tree = build_tree(xml)?;
    let root = tree.child("animations").ok_or(FormatError::MissingRoot)?;

    let header_node = root
        .child("header")
        .ok_or(FormatError::MissingElement("header"))?;
    let header = Header {
        author: header_node.text_of("author", ""),
        title: header_node.text_of("title", ""),
        petname: header_node.text_of("petname", "Pet"),
        version: header_node.text_of("version", "0"),
        info: header_node.text_of("info", ""),
        application: header_node.int_of("application", 1),
        icon: header_node.text_of("icon", ""),
    };

    let image_node = root
        .child("image")
        .ok_or(FormatError::MissingElement("image"))?;
    let image = ImageDef {
        tiles_x: image_node.int_of("tilesx", 1).max(1) as u32,
        tiles_y: image_node.int_of("tilesy", 1).max(1) as u32,
        png_base64: image_node.text_of("png", "").split_whitespace().collect(),
        transparency: image_node.text_of("transparency", "Magenta"),
    };

    let spawns = root
        .child("spawns")
        .map(|n| {
            n.children_named("spawn")
                .map(|s| Spawn {
                    id: s.attr_int("id", 0),
                    probability: s.attr_int("probability", 100),
                    x: PetValue::new(s.text_of("x", "0")),
                    y: PetValue::new(s.text_of("y", "0")),
                    next: s.int_of("next", 1),
                })
                .collect()
        })
        .unwrap_or_default();

    // Le conteneur d'animations porte le même nom que la racine.
    let animations: Vec<Animation> = root
        .child("animations")
        .map(|n| {
            n.children_named("animation")
                .map(|a| Animation {
                    id: a.attr_int("id", 0),
                    name: a.text_of("name", ""),
                    start: a
                        .child("start")
                        .map(parse_movement)
                        .unwrap_or_else(|| parse_movement(&Node::default())),
                    end: a.child("end").map(parse_movement),
                    sequence: a
                        .child("sequence")
                        .map(parse_sequence)
                        .unwrap_or_else(|| parse_sequence(&Node::default())),
                    border: parse_next_list(a.child("border")),
                    gravity: parse_next_list(a.child("gravity")),
                })
                .collect()
        })
        .unwrap_or_default();

    let childs = root
        .child("childs")
        .map(|n| {
            n.children_named("child")
                .map(|c| Child {
                    animation_id: c.attr_int("animationid", 0),
                    x: PetValue::new(c.text_of("x", "0")),
                    y: PetValue::new(c.text_of("y", "0")),
                    next: c.int_of("next", 1),
                })
                .collect()
        })
        .unwrap_or_default();

    let sounds = root
        .child("sounds")
        .map(|n| {
            n.children_named("sound")
                .map(|s| Sound {
                    animation_id: s.attr_int("animationid", 0),
                    probability: s.int_of("probability", 100),
                    loop_count: s.int_of("loop", 0),
                    base64: s.text_of("base64", ""),
                })
                .collect()
        })
        .unwrap_or_default();

    if animations.is_empty() {
        return Err(FormatError::MissingElement("animations"));
    }

    Ok(PetDefinition {
        header,
        image,
        spawns,
        animations,
        childs,
        sounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>4</tilesx><tilesy>2</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>0</x><y>0</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>2</x><y>0</y><interval>100</interval></start>
          <sequence repeat="3" repeatfrom="1"><frame>0</frame><frame>1</frame>
            <action>flip</action><next probability="50" only="taskbar">2</next></sequence>
          <border><next probability="100">3</next></border>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    #[test]
    fn applique_les_valeurs_par_defaut() {
        let pet = parse_pet(MINIMAL).unwrap();
        let anim = &pet.animations[0];
        assert_eq!(anim.start.offset_y, 0);
        assert_eq!(anim.start.opacity, 1.0);
        assert_eq!(pet.image.transparency, "Magenta");
    }

    #[test]
    fn end_absent_reste_absent() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert!(pet.animations[0].end.is_none());
    }

    #[test]
    fn lit_les_frames_dans_l_ordre() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.frames, vec![0, 1]);
    }

    #[test]
    fn lit_l_action_flip() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.action.as_deref(), Some("flip"));
    }

    #[test]
    fn lit_repeat_comme_expression() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.repeat.compute, "3");
        assert_eq!(pet.animations[0].sequence.repeat_from, 1);
    }

    #[test]
    fn lit_les_drapeaux_only() {
        let pet = parse_pet(MINIMAL).unwrap();
        let next = &pet.animations[0].sequence.next[0];
        assert_eq!(next.id, 2);
        assert_eq!(next.probability, 50);
        assert_eq!(next.only, OnlyFlags::TASKBAR);
    }

    #[test]
    fn separe_border_de_sequence_next() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].border.len(), 1);
        assert_eq!(pet.animations[0].border[0].id, 3);
        assert!(pet.animations[0].gravity.is_empty());
    }

    #[test]
    fn accepte_l_ordre_libre_des_sous_elements() {
        let inverse = MINIMAL.replace(
            "<start><x>2</x><y>0</y><interval>100</interval></start>",
            "<start><interval>100</interval><y>0</y><x>2</x></start>",
        );
        let pet = parse_pet(&inverse).unwrap();
        assert_eq!(pet.animations[0].start.x.compute, "2");
        assert_eq!(pet.animations[0].start.interval.compute, "100");
    }

    #[test]
    fn horizontal_plus_vaut_horizontal() {
        assert_eq!(OnlyFlags::parse("horizontal+"), OnlyFlags::HORIZONTAL);
        assert_eq!(OnlyFlags::parse("none"), OnlyFlags::NONE);
        assert_eq!(OnlyFlags::parse(""), OnlyFlags::NONE);
    }
}
