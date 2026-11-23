use super::device_class::DeviceClass;

const A2DP: &str = "0000110b-0000-1000-8000-00805f9b34fb";
const HFP: &str = "0000111e-0000-1000-8000-00805f9b34fb";
const HID: &str = "00001124-0000-1000-8000-00805f9b34fb";
const HOG: &str = "00001812-0000-1000-8000-00805f9b34fb";

/// Best-effort UI class from advertised service UUIDs.
pub fn class_from_uuids(uuids: &[String]) -> DeviceClass {
    let lower: Vec<_> = uuids.iter().map(|u| u.to_ascii_lowercase()).collect();
    if lower.iter().any(|u| u == A2DP) {
        return DeviceClass::Headphones;
    }
    if lower.iter().any(|u| u == HFP) {
        return DeviceClass::Phone;
    }
    if lower.iter().any(|u| u == HID || u == HOG) {
        return DeviceClass::Keyboard;
    }
    DeviceClass::Other
}
