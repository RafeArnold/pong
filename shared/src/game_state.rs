pub const GAME_HEIGHT: u8 = 11;
pub const GAME_WIDTH: u8 = 51;
pub const PADDLE_HEIGHT: u8 = 5;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct GameState {
    pub left_paddle: u8,
    pub right_paddle: u8,
    pub ball: Ball,
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Ball {
    pub x: u8,
    pub y: u8,
    pub moving_right: bool,
    pub moving_down: bool,
}
