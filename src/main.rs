/* PROJECTO: Victron Solar Monitor
 * REQUISITO: Apenas Linux (Tumbleweed)
 */

#[cfg(not(target_os = "linux"))]
compile_error!("SISTEMA NÃO SUPORTADO: Este software é protegido para ser excutado apenas no Linux.");

use std::io::{BufRead, BufReader};
use std::time::Duration;
use serialport;
use rusqlite::{params, Connection};
use chrono::Local;

#[derive(Debug, Default)]
struct MpptData {
    voltage_mv: i32,
    current_ma: i32,
    panel_voltage_mv: i32,
    panel_power_w: i32,
    yield_today_wh: i32,
    max_power_today_w: i32,
    v_bat_min_mv: i32,
    v_bat_max_mv: i32,
}

fn main() {
    let db_path = "vrust.db";
    let porta_serie = "/dev/ttyUSB0";

    println!("A iniciar a monitorização...");

    let port = serialport::new(porta_serie, 19200)
    .timeout(Duration::from_millis(1000))
    .open();

    match port {
        Ok(port) => {
            let mut reader = BufReader::new(port);
            let mut data = MpptData::default();

            loop {
                let mut linha = String::new();
                if reader.read_line(&mut linha).is_ok() {
                    let trim_linha = linha.trim();

                    // Se a linha for o Checksum ou estiver vazia, ignoramos e redesenhamos
                    if trim_linha.starts_with("Checksum") || trim_linha.is_empty() {
                        exibir_interface(&data);

                        // LÓGICA DE DECISÃO: Noite vs Dia
                        if data.panel_voltage_mv < 1000 && data.yield_today_wh > 0 {
                            println!("\n[SISTEMA]: Noite detetada. A Gravar o resumo diário...");
                            gravar_no_sqlite(db_path, &data);
                            println!("[SISTEMA]: Concluído. Até amanhã!");
                            break;
                        }
                        continue;
                    }

                    // Parsing robusto: procuramos o TAB
                    let partes: Vec<&str> = trim_linha.split('\t').collect();
                    if partes.len() == 2 {
                        let label = partes[0].trim();
                        let val = partes[1].trim();

                        match label {
                            "V" => data.voltage_mv = val.parse().unwrap_or(data.voltage_mv),
                            "I" => data.current_ma = val.parse().unwrap_or(data.current_ma),
                            "VPV" => data.panel_voltage_mv = val.parse().unwrap_or(data.panel_voltage_mv),
                            "PPV" => data.panel_power_w = val.parse().unwrap_or(data.panel_power_w),
                            "H19" => data.yield_today_wh = val.parse().unwrap_or(data.yield_today_wh),
                            "H21" => data.max_power_today_w = val.parse().unwrap_or(data.max_power_today_w),
                            "H22" => data.v_bat_min_mv = val.parse().unwrap_or(data.v_bat_min_mv),
                            "H23" => data.v_bat_max_mv = val.parse().unwrap_or(data.v_bat_max_mv),
                            _ => {}
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("Erro ao abrir porta: {}", e),
    }
}

fn exibir_interface(d: &MpptData) {
    // Limpa o ecrã
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);

    let modo = if d.panel_voltage_mv > 1000 { "SOLAR ATIVO (DIA)" } else { "A AGUARDAR FICHEIRO (NOITE)" };

    println!("==========================================");
    println!("     MONITOR MPPT - MODO: {} ", modo);
    println!("     Hora: {} ", Local::now().format("%H:%M:%S"));
    println!("==========================================");
    println!(" Bateria: {:.2} V  | Corrente: {:.2} A", d.voltage_mv as f32 / 1000.0, d.current_ma as f32 / 1000.0);
    println!(" Painel:  {:.2} V  | Potência: {} W", d.panel_voltage_mv as f32 / 1000.0, d.panel_power_w);
    println!("------------------------------------------");
    println!(" Yield Hoje: {} Wh | Pico: {} W", d.yield_today_wh, d.max_power_today_w);
    println!(" Bat Min/Max: {:.2}V / {:.2}V", d.v_bat_min_mv as f32 / 1000.0, d.v_bat_max_mv as f32 / 1000.0);
    println!("==========================================");
}

fn gravar_no_sqlite(path: &str, d: &MpptData) {
    if let Ok(conn) = Connection::open(path) {
        let hoje = Local::now().format("%Y-%m-%d").to_string();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO historico_solar (data, yield_total_wh, potencia_max_w, v_bat_min_mv, v_bat_max_mv)
        VALUES (?1, ?2, ?3, ?4, ?5)",
                             params![hoje, d.yield_today_wh, d.max_power_today_w, d.v_bat_min_mv, d.v_bat_max_mv],
        );
    }
}
