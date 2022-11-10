use std::{process::ExitCode, time::Duration};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serialport::SerialPort;

type L7Sdu = [u8;8];

fn main() -> ExitCode {
    let args = CliArgs::parse();
    match serialport::new(&args.device, args.baud_rate)
        .timeout(Duration::from_millis(args.timeout))
        .open()
        .and_then(|s| run(args, s))
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error({:?}): {}", e.kind, e.description);
            ExitCode::FAILURE
        }
    }
}

pub fn run(args: CliArgs, mut serial: Box<dyn SerialPort>) -> Result<(), serialport::Error> {
    let mut msg = [0u8;16];
    match args.cmd {
        Command::Raw(Raw { bytes }) => {
            if bytes.len() != 16 {
                eprintln!(
                    "WARNING: The message is incomplete or too long.\n\
                    One message consists of 16 bytes"
                );
            }

            serial.write_all(&bytes)?;
            serial.read_exact(&mut msg)?;
        }
        Command::SetLed(set_led) => {
            let bytes = MsgBuilder::new(args.id, 100, set_led.as_sdu()).build();
            if args.echo {
                eprintln!("MSG: {:?}", bytes);
            }
            
            serial.write_all(&bytes)?;
            serial.read_exact(&mut msg)?;
        }
        Command::ReadButtonPresses => {
            let bytes = MsgBuilder::new(args.id, 101, L7Sdu::default()).build();
            if args.echo {
                eprintln!("MSG: {:?}", bytes);
            }
            
            serial.write_all(&bytes)?;
            serial.read_exact(&mut msg)?;

            println!("Button Presses: {}", msg[13]);
        }
        Command::ReadUid => todo!(),
    }
    
    if args.echo {
        eprintln!("Response: {:?}", msg);
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct MsgBuilder {
    pub to: u8,
    pub from: u8,
    pub hops: u8,
    pub version: u8,
    pub opcode: u8,
    pub l7_sdu: [u8; 8],
}

impl MsgBuilder {
    pub fn new(to: u8, opcode: u8, l7_sdu: L7Sdu) -> Self {
        Self {
            to,
            from: 0,
            version: 4,
            hops: 0,
            opcode,
            l7_sdu,
        }
    }

    pub fn build(self) -> [u8; 16] {
        let check_sum = ![self.to, self.from, self.version, self.hops, self.opcode]
            .into_iter()
            .chain(self.l7_sdu)
            .fold(0u8, |i,acc| i.wrapping_add(acc));
        
        self.build_with_checksum(check_sum)
    }

    pub fn build_with_checksum(self, check_sum: u8) -> [u8; 16] {
        [
            0,
            self.to,
            self.from,
            self.version,
            self.hops,
            self.opcode,
            self.l7_sdu[0],
            self.l7_sdu[1],
            self.l7_sdu[2],
            self.l7_sdu[3],
            self.l7_sdu[4],
            self.l7_sdu[5],
            self.l7_sdu[6],
            self.l7_sdu[7],
            check_sum,
            0,
        ]
    }
}

// fn calc_crc<I: Iterator<Item=u8>>(iter: &I) {
    
// }

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    device: String,
    id: u8,
    #[arg(short, long)]
    echo: bool,
    #[arg(short, long, default_value_t = 115_200)]
    baud_rate: u32,
    /// Timeout in milli seconds
    #[arg(short, long, default_value_t = 500)]
    timeout: u64,
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Raw(Raw),
    SetLed(SetLed),
    ReadButtonPresses,
    ReadUid,
}

#[derive(Args, Debug, Clone)]
pub struct Raw {
    bytes: Vec<u8>,
}

#[derive(Args, Debug, Clone, Copy)]
pub struct SetLed {
    on: LedState,
}

impl SetLed {
    pub fn as_sdu(self) -> [u8;8] {
        let mut sdu = L7Sdu::default();
        if self.on == LedState::On {
            sdu[7] = 1;
        }
        
        sdu
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LedState {
    On,
    Off,
}

