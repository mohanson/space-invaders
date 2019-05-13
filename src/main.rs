use std::io::Read;

fn main() {
    let mut mem = i8080::Linear::new();
    let mut file = std::fs::File::open("/src/i8080/res/cpu_tests/8080PRE.COM").unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    mem.data[0x0100..(buf.len() + 0x0100)].clone_from_slice(&buf[..]);

    let mut cpu = i8080::Cpu::power_up(Box::new(mem));

    cpu.mem.set(0x0005, 0xc9);
    cpu.reg.pc = 0x0100;

    loop {
        cpu.next();
        if cpu.reg.pc == 0x76 {
            panic!("")
        }
        if cpu.reg.pc == 0x05 {
            if cpu.reg.c == 0x09 {
                let mut a = cpu.reg.get_de();
                loop {
                    let c = cpu.mem.get(a);
                    if c as char == '$' {
                        break;
                    } else {
                        a += 1;
                    }
                    print!("{}", c as char);
                }
            }
            if cpu.reg.c == 0x02 {
                print!("{}", cpu.reg.e as char);
            }
        }
        if cpu.reg.pc == 0x00 {
            println!("");
            println!("");
            break;
        }
    }

}
