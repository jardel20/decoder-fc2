// Human explanations are presentation only; original diagnostic codes stay intact.
const MESSAGES: Record<string, string> = {
  calendar_all_zero_raw_values_preserved:
    "Data e hora não estão preenchidas neste registro.",
  calendar_invalid_raw_values_preserved:
    "A data registrada é inválida e não pôde ser interpretada.",
  time_invalid_raw_values_preserved:
    "O horário registrado é inválido e não pôde ser interpretado.",
  latitude_nonfinite_raw_bits_preserved:
    "A latitude não contém uma coordenada utilizável.",
  longitude_nonfinite_raw_bits_preserved:
    "A longitude não contém uma coordenada utilizável.",
  latitude_out_of_range_raw_bits_preserved:
    "A latitude está fora do intervalo permitido (−90° a 90°).",
  longitude_out_of_range_raw_bits_preserved:
    "A longitude está fora do intervalo permitido (−180° a 180°).",
};
export const warningMessage = (code: string) =>
  MESSAGES[code] ?? `Aviso de leitura sem descrição disponível: ${code}`;
