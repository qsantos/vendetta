//! The confirmation window (`ClsIntConfirmation`), for attacking one's allies: modal, with two
//! buttons, Oui (Enter) and Non (Esc).

use super::{Hud, Input};
use crate::assets::Textures;
use crate::layout::{screen_h, screen_w};
use macroquad::prelude::*;
use sim::world::Order;

pub const WINDOW: &str = "Images/Fenetres/Fenetre-Confirmation.bmp";
pub const BUTTONS: &str = "Images/Boutons/Menu.bmp";

const SIZE: (i32, i32) = (320, 160);
/// The buttons' top-left corners in the window, and their side.
const YES: (i32, i32) = (64, 100);
const NO: (i32, i32) = (320 - 32 - 64, 100);
const SIDE: i32 = 32;
/// The message's top-left corner in the window.
const MESSAGE: (i32, i32) = (16, 16);

impl Hud {
    /// `Afficher_Fenetre` for the question "attack your allies?" (`ToucheAide`, button 15).
    pub(super) fn ask_attack_allies(&mut self) {
        self.confirm = true;
    }

    /// Draws the window, with the buttons lit under the mouse and their tooltips, and takes its
    /// answer: Oui sends the order.
    pub(super) fn confirmation(&mut self, textures: &Textures, player: usize, input: &Input, orders: &mut Vec<Order>) {
        let (x, y) = ((screen_w() - SIZE.0) / 2, (screen_h() - SIZE.1) / 2);
        if let Some(tex) = textures.get(WINDOW) {
            tex.draw(x as f32, y as f32, WHITE);
        }
        let m = input.mouse;
        let over = |(bx, by): (i32, i32)| super::buttons::inside(m, x + bx, y + by, SIDE, SIDE);
        let mut tip = "";
        for (col, at, label) in [(2, YES, &self.strings.yes), (3, NO, &self.strings.no)] {
            let hover = over(at);
            if let Some(tex) = textures.get(BUTTONS) {
                let src = Rect::new((col * SIDE) as f32, (hover as i32 * SIDE) as f32, SIDE as f32, SIDE as f32);
                tex.draw_ex((x + at.0) as f32, (y + at.1) as f32, WHITE, DrawTextureParams { source: Some(src), ..Default::default() });
            }
            if hover {
                tip = label;
            }
        }
        self.text(x + MESSAGE.0, y + MESSAGE.1, &self.strings.attack_allies, BLACK);
        let tip = tip.to_owned();
        self.tooltip(m, &tip);
        let yes = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || input.left_pressed && over(YES);
        let no = is_key_pressed(KeyCode::Escape) || input.left_pressed && over(NO);
        if yes {
            orders.push(Order::AttackAllies { perso: player });
        }
        if yes || no {
            self.confirm = false;
        }
    }
}
