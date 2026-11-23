use gtk::prelude::*;
use gtk4 as gtk;

use super::props::{IconTileProps, TileSize};
use crate::shared::icon::{self, IconProps};

pub fn build(props: &IconTileProps) -> gtk::Widget {
    let (size_class, glyph) = match props.size {
        TileSize::Small => ("icon-tile--small", 18),
        TileSize::Large => ("icon-tile--large", 34),
    };
    let tile = gtk::CenterBox::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes([
            "icon-tile".to_owned(),
            size_class.to_owned(),
            format!("icon-tile--{}", props.tone.suffix()),
        ])
        .build();
    tile.set_center_widget(Some(&icon::build(&IconProps {
        icon: props.icon,
        size: glyph,
    })));
    tile.upcast()
}
