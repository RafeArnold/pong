use std::{
    cmp::min,
    io::{stdin, stdout, Stdout, StdoutLock, Write},
    sync::{
        atomic::{AtomicU16, AtomicU64, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use termion::{
    clear,
    cursor::{Down, HideCursor, Left, Right, Up},
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
};

const HEIGHT: u16 = 11;
const WIDTH: u16 = 51;
const PADDLE_HEIGHT: u16 = 5;

fn main() {
    let raw_terminal = HideCursor::from(stdout()).into_raw_mode().unwrap();
    raw_terminal.activate_raw_mode().unwrap();

    let left_paddle = Arc::new(AtomicU16::new(0));
    let right_paddle = Arc::new(AtomicU16::new(0));
    let ball = Ball {
        x: (WIDTH + 1) / 2,
        y: (HEIGHT + 1) / 2,
        moving_right: true,
        moving_down: true,
    };
    let ball = Arc::new(AtomicU64::new(serialize_ball(ball)));

    draw(raw_terminal.lock(), &left_paddle, &right_paddle, &ball);

    let (game_over_tx, game_over_rx) = channel();

    spawn_ball_mover(&left_paddle, &right_paddle, &ball, game_over_tx.clone());
    spawn_key_handler(&left_paddle, &right_paddle, &ball, game_over_tx);
    handle_game_over(game_over_rx, raw_terminal).join().unwrap();
}

fn spawn_ball_mover(
    left_paddle: &Arc<AtomicU16>,
    right_paddle: &Arc<AtomicU16>,
    ball: &Arc<AtomicU64>,
    game_over_tx: Sender<GameOver>,
) -> JoinHandle<()> {
    let left_paddle = Arc::clone(left_paddle);
    let right_paddle = Arc::clone(right_paddle);
    let ball = Arc::clone(ball);
    spawn(move || {
        let stdout = stdout();
        loop {
            sleep(Duration::from_millis(100));
            let update_result = ball.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |ball| {
                let mut ball = deserialize_ball(ball);
                if ball.x == 1 {
                    let left_paddle = left_paddle.load(Ordering::Relaxed);
                    if left_paddle > ball.y || left_paddle + PADDLE_HEIGHT <= ball.y {
                        return None;
                    } else {
                        ball.moving_right = !ball.moving_right;
                    }
                }
                if ball.x == WIDTH - 2 {
                    let right_paddle = right_paddle.load(Ordering::Relaxed);
                    if right_paddle > ball.y || right_paddle + PADDLE_HEIGHT <= ball.y {
                        return None;
                    } else {
                        ball.moving_right = !ball.moving_right;
                    }
                }
                if ball.y == 0 || ball.y == HEIGHT - 1 {
                    ball.moving_down = !ball.moving_down;
                }
                if ball.moving_right {
                    ball.x += 1;
                } else {
                    ball.x -= 1;
                }
                if ball.moving_down {
                    ball.y += 1;
                } else {
                    ball.y -= 1;
                }
                Some(serialize_ball(ball))
            });
            if let Some(prev_ball) = update_result.err().map(deserialize_ball) {
                game_over_tx
                    .send(if prev_ball.x == 1 {
                        GameOver::RightWon
                    } else {
                        GameOver::LeftWon
                    })
                    .unwrap();
                return;
            }
            let mut stdout = stdout.lock();
            write!(stdout, "{}", Up(HEIGHT + 1)).unwrap();
            draw(stdout, &left_paddle, &right_paddle, &ball);
        }
    })
}

fn spawn_key_handler(
    left_paddle: &Arc<AtomicU16>,
    right_paddle: &Arc<AtomicU16>,
    ball: &Arc<AtomicU64>,
    game_over_tx: Sender<GameOver>,
) -> JoinHandle<()> {
    let left_paddle = Arc::clone(left_paddle);
    let right_paddle = Arc::clone(right_paddle);
    let ball = Arc::clone(ball);
    spawn(move || {
        let stdin = stdin();
        let stdout = stdout();
        for key in stdin.keys() {
            let key = key.unwrap();
            match key {
                Key::Ctrl('c') => {
                    game_over_tx.send(GameOver::CtrlC).unwrap();
                }
                Key::Char('w') => {
                    let _ = left_paddle.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                        Some(x.saturating_sub(1))
                    });
                }
                Key::Char('s') => {
                    let _ = left_paddle.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                        Some(min(x + 1, HEIGHT - PADDLE_HEIGHT))
                    });
                }
                Key::Up => {
                    let _ = right_paddle.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                        Some(x.saturating_sub(1))
                    });
                }
                Key::Down => {
                    let _ = right_paddle.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                        Some(min(x + 1, HEIGHT - PADDLE_HEIGHT))
                    });
                }
                _ => continue,
            }
            let mut stdout = stdout.lock();
            write!(stdout, "{}", Up(HEIGHT + 1)).unwrap();
            draw(stdout, &left_paddle, &right_paddle, &ball);
        }
    })
}

fn handle_game_over(
    game_over_rx: Receiver<GameOver>,
    stdout: RawTerminal<HideCursor<Stdout>>,
) -> JoinHandle<()> {
    spawn(move || {
        let game_over = game_over_rx.recv().unwrap();
        stdout.suspend_raw_mode().unwrap();
        println!();
        match game_over {
            GameOver::CtrlC => {}
            GameOver::LeftWon => println!("left won!"),
            GameOver::RightWon => println!("right won!"),
        };
    })
}

fn draw(mut w: StdoutLock, left_paddle: &AtomicU16, right_paddle: &AtomicU16, ball: &AtomicU64) {
    let left_paddle = left_paddle.load(Ordering::Relaxed);
    let right_paddle = right_paddle.load(Ordering::Relaxed);
    let ball = ball.load(Ordering::Relaxed);
    let ball = deserialize_ball(ball);
    clear(&mut w);
    write!(w, "{}{}o", Right(ball.x + 1), Down(ball.y + 1)).unwrap();
    write!(w, "{}{}", Left(ball.x + 1), Up(ball.y + 1)).unwrap();
    draw_barrier(&mut w);
    writeln!(w).unwrap();
    draw_paddle(&mut w, left_paddle);
    draw_barrier(&mut w);
    write!(w, "{}{}", Up(HEIGHT), Right(WIDTH - 1)).unwrap();
    draw_paddle(&mut w, right_paddle);
    write!(w, "{}", Left(WIDTH)).unwrap();
    w.flush().unwrap();
}

fn clear<W: Write>(w: &mut W) {
    for _ in 0..HEIGHT + 1 {
        writeln!(w, "{}", clear::CurrentLine).unwrap();
    }
    write!(w, "{}", Up(HEIGHT + 1)).unwrap();
}

fn draw_barrier<W: Write>(w: &mut W) {
    for _ in 0..WIDTH {
        write!(w, "-").unwrap();
    }
    write!(w, "{}", Left(WIDTH)).unwrap();
}

fn draw_paddle<W: Write>(w: &mut W, paddle: u16) {
    for _ in 0..paddle {
        write!(w, "{}", Down(1)).unwrap();
    }
    for _ in 0..PADDLE_HEIGHT {
        write!(w, "|{}{}", Left(1), Down(1)).unwrap();
    }
    for _ in 0..HEIGHT - PADDLE_HEIGHT - paddle {
        write!(w, "{}", Down(1)).unwrap();
    }
}

fn deserialize_ball(ball: u64) -> Ball {
    Ball {
        x: (ball >> 48) as u16,
        y: (ball >> 32) as u16,
        moving_right: (ball >> 16) as u16 == 1,
        moving_down: ball as u16 == 1,
    }
}

fn serialize_ball(ball: Ball) -> u64 {
    (ball.x as u64) << 48
        | (ball.y as u64) << 32
        | (ball.moving_right as u64) << 16
        | ball.moving_down as u64
}

struct Ball {
    x: u16,
    y: u16,
    moving_right: bool,
    moving_down: bool,
}

enum GameOver {
    CtrlC,
    LeftWon,
    RightWon,
}
