//! Human explanations are presentation only; original diagnostic codes stay intact.
use eframe::egui;
use std::borrow::Cow;

pub fn message(code: &str) -> Cow<'_, str> {
    match code {
        "calendar_all_zero_raw_values_preserved" => {
            "Data e hora não estão preenchidas neste registro.".into()
        }
        "calendar_invalid_raw_values_preserved" => {
            "A data registrada é inválida e não pôde ser interpretada.".into()
        }
        "time_invalid_raw_values_preserved" => {
            "O horário registrado é inválido e não pôde ser interpretado.".into()
        }
        "latitude_nonfinite_raw_bits_preserved" => {
            "A latitude não contém uma coordenada utilizável.".into()
        }
        "longitude_nonfinite_raw_bits_preserved" => {
            "A longitude não contém uma coordenada utilizável.".into()
        }
        "latitude_out_of_range_raw_bits_preserved" => {
            "A latitude está fora do intervalo permitido (−90° a 90°).".into()
        }
        "longitude_out_of_range_raw_bits_preserved" => {
            "A longitude está fora do intervalo permitido (−180° a 180°).".into()
        }
        _ => format!("Aviso de leitura sem descrição disponível: {code}").into(),
    }
}

pub fn tooltip(ui: &mut egui::Ui, id: u32, warnings: &[String]) {
    ui.set_max_width(360.0);
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.strong(format!("Avisos do registro {id}"));
    ui.separator();
    for code in warnings {
        ui.label(message(code).as_ref());
    }
    ui.small("Os valores originais foram preservados no arquivo.");
    egui::CollapsingHeader::new("Detalhes técnicos").show(ui, |ui| {
        for code in warnings {
            ui.monospace(code);
        }
    });
}
