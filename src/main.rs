use i8080::Memory;
use rodio::Source;
use std::io::Read;

#[derive(Default)]
struct DeviceIO {
    inp0: u8,       // INPUTS (Mapped in hardware but never used by the code)
    inp1: u8,       // INPUTS
    inp2: u8,       // INPUTS
    shft_in: u8,    // bit shift register read
    shft_amnt: u8,  // shift amount (3 bits)
    sound1: u8,     // sound bits
    shft_data: u16, // shift data
    sound2: u8,     // sound bits
    watchdog: u8,   // watch-dog
}

impl DeviceIO {
    fn power_up() -> Self {
        Self::default()
    }
}

const DISPLAY_W: usize = 224;
const DISPLAY_H: usize = 256;

struct Display {
    raster: Vec<u32>,
    window: minifb::Window,
}

impl Display {
    fn power_up() -> Display {
        let mut option = minifb::WindowOptions::default();
        option.resize = true;
        option.scale = minifb::Scale::X2;
        let window = minifb::Window::new("github.com/mohanson/i8080", DISPLAY_W, DISPLAY_H, option).unwrap();
        Display { raster: vec![0x00ff_ffff; DISPLAY_W * DISPLAY_H * 2], window }
    }

    fn draw_pixel(&mut self, data: &[u8]) {
        for (i, byte) in data.iter().enumerate() {
            let y = i as isize * 8 / 256;

            for shift in 0..8 {
                let x = ((i * 8) % 256 as usize + shift as usize) as isize;

                // Rotate frame buffer 90 deg
                let new_x = y as isize;
                let new_y = (-x as isize + 256) - 1;

                let pixel = if byte.wrapping_shr(shift) & 1 == 0 {
                    0xFF00_0000 // Alpha
                } else if x <= 63 && (x >= 15 || x <= 15 && y >= 20 && y <= 120) {
                    0xFF00_FF00 // Green
                } else if x >= 200 && x <= 220 {
                    0xFF00_00FF // Red
                } else {
                    0xFFFF_FFFF // Black
                };
                self.raster[DISPLAY_W * new_y as usize + new_x as usize] = pixel;
            }
        }
        self.window.update_with_buffer(&self.raster, DISPLAY_W, DISPLAY_H).unwrap();
    }
}

struct Sounder {
    stream: rodio::OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    wavs: [Vec<u8>; 10],
}

impl Sounder {
    fn power_up(snd: impl AsRef<std::path::Path>) -> Self {
        let res = snd.as_ref().to_path_buf();
        let get = |x| {
            let mut res = res.clone();
            res.push(x);
            std::fs::read(res).unwrap()
        };
        let mut wavs = [vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
        for i in 0..10 {
            wavs[i as usize] = get(format!("{}.wav", i));
        }
        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        Self { stream, stream_handle, wavs }
    }

    fn play_sound(&self, i: usize) {
        let data = self.wavs[i].clone();
        let cursor = std::io::Cursor::new(data);
        self.stream_handle.play_raw(rodio::Decoder::new_wav(cursor).unwrap().convert_samples()).unwrap();
    }
}

struct Invaders {
    cpu: i8080::Cpu,
    display: Display,
    sounder: Sounder,
    interrupt_addr: u16,
    mem: std::rc::Rc<std::cell::RefCell<i8080::Linear>>,
    io: DeviceIO,
}

impl Invaders {
    fn power_up(rom: impl AsRef<std::path::Path>) -> Self {
        let mem = std::rc::Rc::new(std::cell::RefCell::new(i8080::Linear::new()));
        let mut file = std::fs::File::open(rom).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        mem.borrow_mut().data[0x00..buf.len()].clone_from_slice(&buf[..]);

        Self {
            cpu: i8080::Cpu::power_up(mem.clone()),
            display: Display::power_up(),
            sounder: Sounder::power_up("./res/snd"),
            interrupt_addr: 0x08,
            mem,
            io: DeviceIO::power_up(),
        }
    }

    fn play_sound(&mut self, data: u8, bank: u8) {
        if bank == 1 && data != self.io.sound1 {
            if i8080::bit::get(data, 0) && !i8080::bit::get(self.io.sound1, 0) {
                self.sounder.play_sound(0)
            }
            if i8080::bit::get(data, 1) && !i8080::bit::get(self.io.sound1, 1) {
                self.sounder.play_sound(1)
            }
            if i8080::bit::get(data, 2) && !i8080::bit::get(self.io.sound1, 2) {
                self.sounder.play_sound(2)
            }
            if i8080::bit::get(data, 3) && !i8080::bit::get(self.io.sound1, 3) {
                self.sounder.play_sound(3)
            }
            self.io.sound1 = data;
        }
        if bank == 2 && data != self.io.sound2 {
            if i8080::bit::get(data, 0) && !i8080::bit::get(self.io.sound2, 0) {
                self.sounder.play_sound(4)
            }
            if i8080::bit::get(data, 1) && !i8080::bit::get(self.io.sound2, 1) {
                self.sounder.play_sound(5)
            }
            if i8080::bit::get(data, 2) && !i8080::bit::get(self.io.sound2, 2) {
                self.sounder.play_sound(6)
            }
            if i8080::bit::get(data, 3) && !i8080::bit::get(self.io.sound2, 3) {
                self.sounder.play_sound(7)
            }
            if i8080::bit::get(data, 4) && !i8080::bit::get(self.io.sound2, 4) {
                self.sounder.play_sound(8)
            }
            self.io.sound2 = data;
        }
    }

    fn handle_joypad(&mut self) {
        if self.display.window.is_key_down(minifb::Key::Escape) {
            std::process::exit(0);
        }
        let keys = vec![
            (minifb::Key::C, 1, 0),     // Insert a coin
            (minifb::Key::Key2, 1, 1),  // P2 start
            (minifb::Key::Key1, 1, 2),  // P1 start
            (minifb::Key::W, 1, 4),     // P1 shot
            (minifb::Key::Up, 2, 4),    // P2 shot
            (minifb::Key::Q, 1, 5),     // P1 left
            (minifb::Key::Left, 2, 5),  // P2 left
            (minifb::Key::E, 1, 6),     // P1 right
            (minifb::Key::Right, 2, 6), // P2 right
            (minifb::Key::T, 2, 2),     // tilt
        ];
        for (k, inpi, biti) in &keys {
            let func = |x| {
                if self.display.window.is_key_down(*k) {
                    i8080::bit::set(x, *biti as usize)
                } else {
                    i8080::bit::clr(x, *biti as usize)
                }
            };
            match inpi {
                1 => self.io.inp1 = func(self.io.inp1),
                2 => self.io.inp2 = func(self.io.inp2),
                _ => {}
            }
        }
    }

    fn next(&mut self) -> u32 {
        let opcode = self.mem.borrow().get(self.cpu.reg.pc);
        match opcode {
            0xdb => {
                let port = self.mem.borrow().get(self.cpu.reg.pc + 1);
                let r = match port {
                    0 => self.io.inp0,
                    1 => self.io.inp1,
                    2 => self.io.inp2,
                    3 => {
                        self.io.shft_in = ((self.io.shft_data >> u16::from(8 - self.io.shft_amnt)) & 0xff) as u8;
                        self.io.shft_in
                    }
                    _ => panic!(""),
                };
                self.cpu.reg.a = r;
            }
            0xd3 => {
                let port = self.mem.borrow().get(self.cpu.reg.pc + 1);
                match port {
                    2 => self.io.shft_amnt = self.cpu.reg.a & 0x7,
                    3 => {
                        self.play_sound(self.cpu.reg.a, 1);
                    }
                    4 => self.io.shft_data = u16::from(self.cpu.reg.a) << 8 | self.io.shft_data >> 8,
                    5 => {
                        self.play_sound(self.cpu.reg.a, 2);
                    }
                    6 => self.io.watchdog = self.cpu.reg.a,
                    7 => {}
                    _ => panic!(""),
                }
            }
            _ => {}
        }
        self.cpu.step()
    }

    fn step(&mut self) {
        let mut cycle = 0;
        while cycle < 16667 {
            cycle += self.next();
        }
        self.cpu.inte_handle(self.interrupt_addr);
        self.interrupt_addr = if self.interrupt_addr == 0x08 { 0x10 } else { 0x08 };
        while cycle < 33334 {
            cycle += self.next();
        }
        self.display.draw_pixel(&self.mem.borrow().data[0x2400..0x4000]);
        self.handle_joypad();
    }
}

fn main() {
    let mut invaders = Invaders::power_up("./res/invaders.rom");
    // If this is dropped playback will end & attached OutputStreamHandle will no longer work.
    let _ = invaders.sounder.stream;
    println!("----------------------------------");
    println!("| Welcome to space invaders!     |");
    println!("|                                |");
    println!("|    Press `C` to insert a coin  |");
    println!("|    Press `1` to start player 1 |");
    println!("|    Press `Q` to move left      |");
    println!("|    Press `E` to move right     |");
    println!("|    Press `W` to fire           |");
    println!("|                                |");
    println!("| Good luck                      |");
    println!("----------------------------------");
    loop {
        if !invaders.display.window.is_open() {
            return;
        }
        invaders.step();
    }
}
