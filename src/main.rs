/* PROJECTO: vRust - Victron SmartSolar Private Monitor
 * ESTADO: Estável - Histórico Inteligente sem Repetições
 * CODED: Por Adelino Saldanha aka AS, kernelx64
 */

use std::io::{self, BufRead, BufReader, Write, stdout};
use std::time::Duration;
use serialport;
use rusqlite::{params, Connection};
use chrono::{Local, Duration as ChronoDuration};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    cursor::{Hide, Show},
    terminal::{enable_raw_mode, disable_raw_mode},
    execute, style::{Color, SetForegroundColor, ResetColor},
};

const PORTA: &str = "/dev/ttyUSB0";
const DB_NAME: &str = "vrust.db";

#[derive(Default, Clone)]
struct MpptData {
    v: i32, vpv: i32, ppv: i32, h19: i32, h21: i32,
}

fn main() -> io::Result<()> {
    setup_db();
    let mut stdout = stdout();
    let _ = execute!(stdout, Hide);

    loop {
        desenhar_menu_principal(&mut stdout);

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('1') => monitorizacao_ativa(),
                    KeyCode::Char('2') => visualizar_historico_db(),
                    KeyCode::Char('3') => historico_hardware_e_db(),
                    KeyCode::Char('q') | KeyCode::Char('4') => break,
                    _ => {}
                }
            }
        }
    }

    let _ = execute!(stdout, Show);
    Ok(())
}

fn monitorizacao_ativa() {
    println!("\r\n A verificar ligação ao MPPT Victron...");
    let _ = enable_raw_mode();

    let port_result = serialport::new(PORTA, 19200)
    .timeout(Duration::from_secs(3))
    .open();

    match port_result {
        Ok(port) => {
            println!("\r\n\x1b[92m>>> MONITORIZAÇÃO ATIVA (pressione 'q' para sair) <<<\x1b[0m");
            let mut reader = BufReader::new(port);
            let mut data = MpptData::default();

            loop {
                if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.code == KeyCode::Char('q') { break; }
                    }
                }

                let mut buffer = Vec::new();
                if let Ok(_) = reader.read_until(b'\n', &mut buffer) {
                    let linha = String::from_utf8_lossy(&buffer);
                    let trim_linha = linha.trim();

                    if trim_linha.starts_with("Checksum") {
                        let agora = Local::now().format("%H:%M:%S");
                        print!(
                            "\r[{}] Bat: {:.2}V | Pan: {:.2}V | Pot: {}W | Yield: {:.2}kWh   ",
                            agora,
                            data.v as f32 / 1000.0,
                            data.vpv as f32 / 1000.0,
                            data.ppv,
                            data.h19 as f32 / 100.0,
                        );
                        let _ = io::stdout().flush();
                        salvar_db(&data);
                    }

                    let partes: Vec<&str> = trim_linha.split('\t').collect();
                    if partes.len() == 2 {
                        let val = partes[1].trim();
                        match partes[0].trim() {
                            "V"   => data.v = val.parse().unwrap_or(data.v),
                            "VPV" => data.vpv = val.parse().unwrap_or(data.vpv),
                            "PPV" => data.ppv = val.parse().unwrap_or(data.ppv),
                            "H19" => data.h19 = val.parse().unwrap_or(data.h19),
                            "H21" => data.h21 = val.parse().unwrap_or(data.h21),
                            _ => {}
                        }
                    }
                }
            }
        }
        Err(_) => println!("\r\n Erro: Porta {} não disponível.", PORTA),
    }
    let _ = disable_raw_mode();
}

fn historico_hardware_e_db() {
    println!("\r\n  \x1b[38;2;0;114;198m._______________________________________________________.");
    println!("  | {:^53} |", "HISTÓRICO DE RENDIMENTO (HARDWARE + DB)");
    println!("  |-------------------------------------------------------|");
    println!("  | {:<15} | {:>15} | {:>15} |", "DIA / DATA", "YIELD (Wh)", "PMAX (W)");
    println!("  |-----------------|-----------------|-----------------|");

    let hoje = Local::now();
    let mut vistos = std::collections::HashSet::new();

    // 1. Hardware (Hoje e Ontem)
    if let Ok(port) = serialport::new(PORTA, 19200).timeout(Duration::from_secs(2)).open() {
        let mut reader = BufReader::new(port);
        for _ in 0..150 {
            if vistos.len() >= 4 { break; }
            let mut buffer = Vec::new();
            if let Ok(_) = reader.read_until(b'\n', &mut buffer) {
                let linha = String::from_utf8_lossy(&buffer);
                let partes: Vec<&str> = linha.trim().split('\t').collect();
                if partes.len() == 2 {
                    match partes[0] {
                        "H20" if !vistos.contains("H20") => {
                            print!("  | {:<15} | {:>15} |", hoje.format("%Y-%m-%d"), partes[1]);
                            vistos.insert("H20".to_string());
                        }
                        "H21" if !vistos.contains("H21") => {
                            println!(" {:>15} |", partes[1]);
                            vistos.insert("H21".to_string());
                        }
                        "H22" if !vistos.contains("H22") => {
                            print!("  | {:<15} | {:>15} |", (hoje - ChronoDuration::days(1)).format("%Y-%m-%d"), partes[1]);
                            vistos.insert("H22".to_string());
                        }
                        "H23" if !vistos.contains("H23") => {
                            println!(" {:>15} |", partes[1]);
                            vistos.insert("H23".to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 2. Base de Dados (Restantes dias)
    if let Ok(conn) = Connection::open(DB_NAME) {
        let anteontem = (hoje - ChronoDuration::days(2)).format("%Y-%m-%d").to_string();
        let mut stmt = conn.prepare("SELECT date, yield_wh, peak_w FROM mppt_history WHERE date <= ?1 ORDER BY date DESC LIMIT 28").unwrap();
        let rows = stmt.query_map([anteontem], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?))
        }).unwrap();

        for r in rows {
            if let Ok((d, y, p)) = r {
                println!("  | {:<15} | {:>15} | {:>15} |", d, y, p);
            }
        }
    }

    println!("  |_________________|_________________|_________________|\x1b[0m");
    println!("\r\n  Pressione qualquer tecla para voltar...");
    esperar_tecla();
}

fn visualizar_historico_db() {
    println!("\r\n\x1b[94m=== ARQUIVO COMPLETO NA BASE DE DADOS ===\x1b[0m\n");
    if let Ok(conn) = Connection::open(DB_NAME) {
        let mut stmt = conn.prepare("SELECT date, yield_wh, peak_w FROM mppt_history ORDER BY date DESC LIMIT 30").unwrap();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?))
        }).unwrap();

        for r in rows {
            if let Ok((d, y, p)) = r {
                println!("  {} | {:>7} Wh | {:>4} W", d, y, p);
            }
        }
    }
    println!("\nPressione qualquer tecla para voltar...");
    esperar_tecla();
}

fn desenhar_menu_principal(stdout: &mut io::Stdout) {
    let azul_victron = Color::Rgb { r: 0, g: 114, b: 198 };
    let _ = execute!(stdout, SetForegroundColor(azul_victron));
    println!("\r\n  _______________________________________________________");
    println!(" |   vRust v1.1 - Victron SmartSolar Monitor (Linux)     |");
    println!("  -------------------------------------------------------");
    let _ = execute!(stdout, ResetColor);
    println!("  1. Monitorização em Tempo Real");
    println!("  2. Consultar Base de Dados Local (Arquivo)");
    println!("  3. Ver Histórico de 30 Dias (Hardware + DB)");
    println!("  4. Sair (q)");
    print!("\r\n  Escolha: ");
    let _ = stdout.flush();
}

fn salvar_db(d: &MpptData) {
    if let Ok(conn) = Connection::open(DB_NAME) {
        let hoje = Local::now().format("%Y-%m-%d").to_string();
        let _ = conn.execute(
            "INSERT INTO mppt_history (date, yield_wh, peak_w) VALUES (?1, ?2, ?3)
        ON CONFLICT(date) DO UPDATE SET yield_wh=MAX(yield_wh, excluded.yield_wh), peak_w=MAX(peak_w, excluded.peak_w)",
                             params![hoje, d.h19, d.h21],
        );
    }
}

fn setup_db() {
    let conn = Connection::open(DB_NAME).unwrap();
    let _ = conn.execute("CREATE TABLE IF NOT EXISTS mppt_history (date TEXT PRIMARY KEY, yield_wh INTEGER, peak_w INTEGER)", []);
}

fn esperar_tecla() {
    loop {
        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(k)) = event::read() {
                if k.kind == KeyEventKind::Press { break; }
            }
        }
    }
}
