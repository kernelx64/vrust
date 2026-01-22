/* PROJECTO: vRust - Victron SmartSolar Private Monitor
 * ESTADO: Estável e Simples - Foco na Soberania de Dados
 */

use std::io::{self, BufRead, BufReader, Write, stdout};
use std::time::{Duration, Instant};
use serialport;
use rusqlite::{params, Connection};
use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
    execute, style::{Print, Color, SetForegroundColor, ResetColor, Attribute, SetAttribute},
};

const PORTA: &str = "/dev/ttyUSB0";
const DB_NAME: &str = "vrust.db";

#[derive(Default, Clone)]
struct MpptData {
    v: i32, i: i32, vpv: i32, ppv: i32, h19: i32, h21: i32, h17: i32,
}

fn main() -> io::Result<()> {
    setup_db();
    let mut stdout = stdout();
    execute!(stdout, Hide, Clear(ClearType::All))?;

    loop {
        desenhar_menu_principal(&mut stdout);

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('1') => monitorizacao_ativa(&mut stdout),
                    KeyCode::Char('2') => visualizar_historico(&mut stdout),
                    KeyCode::Char('3') => resumo_mensal(&mut stdout),
                    KeyCode::Char('q') | KeyCode::Char('4') => break,
                    _ => {}
                }
            }
        }
    }

    execute!(stdout, Show, Clear(ClearType::All), MoveTo(0,0))?;
    Ok(())
}

fn monitorizacao_ativa(stdout: &mut io::Stdout) {
    let _ = enable_raw_mode();
    println!("\r\n\x1b[92m>>> MONITORIZAÇÃO ATIVA (q para sair) <<<\x1b[0m");

    let port_result = serialport::new(PORTA, 19200)
    .timeout(Duration::from_secs(3))
    .open();

    if let Ok(port) = port_result {
        let mut reader = BufReader::new(port);
        let mut data = MpptData::default();
        let _inicio = Instant::now();

        loop {
            if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.code == KeyCode::Char('q') { break; }
                }
            }

            let mut buffer = Vec::new();
            if let Ok(n) = reader.read_until(b'\n', &mut buffer) {
                if n == 0 { continue; }
                let linha = String::from_utf8_lossy(&buffer);
                let trim_linha = linha.trim();

                if trim_linha.starts_with("Checksum") {
                    let agora = Local::now().format("%H:%M:%S");
                    // Ajustado conforme a observação: H19 é o valor
                    print!(
                        "\r[{}] Bateria: {:.2}V | Pan: {:.2}V | Pot: {}W | Total produzido: {:.2}kWh   ",
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
                        "I"   => data.i = val.parse().unwrap_or(data.i),
                        "H19" => data.h19 = val.parse().unwrap_or(data.h19),
                        "H21" => data.h21 = val.parse().unwrap_or(data.h21),
                        "H17" => data.h17 = val.parse().unwrap_or(data.h17),
                        _ => {}
                    }
                }
            }
        }
    }
    let _ = disable_raw_mode();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
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

fn desenhar_menu_principal(stdout: &mut io::Stdout) {
    let azul_victron = Color::Rgb { r: 0, g: 114, b: 198 };
    let branco_gelo = Color::Rgb { r: 240, g: 248, b: 255 };

    execute!(stdout, Clear(ClearType::All), MoveTo(0, 1)).unwrap();
    execute!(stdout,
             SetForegroundColor(azul_victron),
             Print("  _______________________________________________________\n"),
             Print(" /                                                       \\\n"),
             SetForegroundColor(branco_gelo),
             SetAttribute(Attribute::Bold),
             Print(" |   vRust v1.0 - Victron SmartSolar Private Monitor     |\n"),
             SetAttribute(Attribute::Reset),
             SetForegroundColor(azul_victron),
             Print(" \\_______________________________________________________/\n"),
             ResetColor,
             SetForegroundColor(Color::DarkGrey),
             Print("  [Soberania de Dados: Base de Dados Local (vrust.db)]\n\n"),
             ResetColor,
             Print("  1. Monitorização em Tempo Real (Single-Line)\n"),
             Print("  2. Consultar Arquivo Local (Últimos 30 dias)\n"),
             Print("  3. Relatório Mensal de Rendimento (kWh)\n"),
             Print("  4. Sair do Sistema (q)\n\n"),
             Print("  Escolha: ")
    ).unwrap();
    stdout.flush().unwrap();
}

fn setup_db() {
    let conn = Connection::open(DB_NAME).unwrap();
    let _ = conn.execute("CREATE TABLE IF NOT EXISTS mppt_history (date TEXT PRIMARY KEY, yield_wh INTEGER, peak_w INTEGER)", []);
}

fn visualizar_historico(stdout: &mut io::Stdout) {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    println!("=== ARQUIVO HISTÓRICO LOCAL ===\n");
    if let Ok(conn) = Connection::open(DB_NAME) {
        let mut stmt = conn.prepare("SELECT date, yield_wh, peak_w FROM mppt_history ORDER BY date DESC LIMIT 15").unwrap();

        // CORREÇÃO AQUI: Removi os tipos complexos do get
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // O "_" diz ao Rust: "eu decido o índice"
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?
            ))
        }).unwrap();

        for r in rows {
            if let Ok((d, y, p)) = r {
                println!("  {} | {:>6} Wh | {:>4} W", d, y, p);
            }
        }
    }
    println!("\nPressione qualquer tecla para voltar...");
    esperar_tecla();
}

fn resumo_mensal(stdout: &mut io::Stdout) {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    println!("\x1b[38;2;0;114;198m=== RELATÓRIO MENSAL (kWh) ===\x1b[0m\r\n");
    if let Ok(conn) = Connection::open(DB_NAME) {
        let mut stmt = conn.prepare("SELECT strftime('%Y-%m', date) as m, SUM(yield_wh) FROM mppt_history GROUP BY m ORDER BY m DESC").unwrap();

        // CORREÇÃO AQUI:
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?
            ))
        }).unwrap();

        for r in rows {
            if let Ok((mes, total)) = r {
                println!("  {:<9} | {:.2} kWh", mes, total as f32 / 1000.0);
            }
        }
    }
    println!("\nPressione qualquer tecla para voltar...");
    esperar_tecla();
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
