//! Patient bar — persistent strip showing the selected patient
//! (DESIGN.md: patient-bar). Hidden until a patient is selected.

use leptos::prelude::*;

use crate::components::icons::IconUser;
use crate::state::AppState;

/// Shows the selected patient; renders nothing until one is chosen.
#[component]
pub fn PatientBar(state: AppState) -> impl IntoView {
    move || {
        let patient = state.patient.get()?;
        let cid = patient
            .cid
            .as_deref()
            .map(mask_cid)
            .unwrap_or_else(|| "—".to_string());
        let dob = patient
            .birth_date
            .map(|d| d.format("%d/%m/%Y").to_string())
            .unwrap_or_else(|| "—".to_string());
        let sex = patient
            .sex
            .as_deref()
            .map(sex_label)
            .unwrap_or_else(|| "—".to_string());
        Some(
            view! {
                <section class="patient-bar">
                    <IconUser class="patient-bar__icon" />
                    <div>
                        <h2 class="patient-bar__name">{patient.full_name_th.clone()}</h2>
                        <div class="patient-bar__meta">
                            <span class="code">{"HN "}{patient.hn.clone()}</span>
                            <span class="code">{"CID "}{cid}</span>
                            <span>{"เกิด "}{dob}</span>
                            <span>{"เพศ "}{sex}</span>
                        </div>
                    </div>
                </section>
            }
            .into_any(),
        )
    }
}

/// CID is always masked on display: `1-XXXX-XXXXX-XX-1` (DESIGN.md).
fn mask_cid(cid: &str) -> String {
    let chars: Vec<char> = cid.chars().collect();
    if chars.len() != 13 {
        return "—".to_string();
    }
    let mut masked = String::with_capacity(17);
    masked.push(chars[0]);
    masked.push('-');
    masked.extend(std::iter::repeat_n('X', 4));
    masked.push('-');
    masked.extend(std::iter::repeat_n('X', 5));
    masked.push('-');
    masked.extend(std::iter::repeat_n('X', 2));
    masked.push('-');
    masked.push(chars[12]);
    masked
}

/// HOSxP sex codes on this instance: `1` = ชาย, `2` = หญิง. Anything else
/// renders as "ไม่ระบุ" rather than leaking a raw code to the operator.
fn sex_label(sex: &str) -> String {
    match sex {
        "1" => "ชาย".to_string(),
        "2" => "หญิง".to_string(),
        _ => "ไม่ระบุ".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_full_national_id() {
        assert_eq!(mask_cid("1101701234567"), "1-XXXX-XXXXX-XX-7");
    }

    #[test]
    fn masks_short_or_empty_cid_to_placeholder() {
        assert_eq!(mask_cid(""), "—");
        assert_eq!(mask_cid("12345"), "—");
    }

    #[test]
    fn maps_known_sex_codes_to_labels() {
        assert_eq!(sex_label("1"), "ชาย");
        assert_eq!(sex_label("2"), "หญิง");
    }

    #[test]
    fn maps_unknown_sex_codes_to_not_specified() {
        assert_eq!(sex_label(""), "ไม่ระบุ");
        assert_eq!(sex_label("9"), "ไม่ระบุ");
    }
}
