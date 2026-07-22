//! Flattened **numeric channel list** for BUS / multi-page reuse.
//! Prefer engineering units on glass (not only 0..1).

use crate::auto::{DriveMode, GearSelect, VehicleSnapshot};

/// One labeled numeric (or short text) field for dense lists.
#[derive(Clone, Debug)]
pub struct Channel {
    pub group: &'static str,
    pub label: &'static str,
    pub value: String,
    pub unit: &'static str,
}

impl Channel {
    pub fn line(&self) -> String {
        if self.unit.is_empty() {
            format!("{:<6} {}", self.label, self.value)
        } else {
            format!("{:<6} {} {}", self.label, self.value, self.unit)
        }
    }
}

fn f1(x: f32) -> String {
    format!("{x:.1}")
}
fn f0(x: f32) -> String {
    format!("{x:.0}")
}

/// All channels currently known on the snapshot (demo or live).
pub fn all_channels(v: &VehicleSnapshot) -> Vec<Channel> {
    let mut c = Vec::with_capacity(64);
    let push = |c: &mut Vec<Channel>, g, l, val: String, u| {
        c.push(Channel {
            group: g,
            label: l,
            value: val,
            unit: u,
        });
    };

    // ENG — dense powerplant numerics
    push(&mut c, "ENG", "RPM", f0(v.rpm), "rpm");
    push(&mut c, "ENG", "RDLN", f0(v.rpm_redline), "rpm");
    push(&mut c, "ENG", "LOAD", f0(v.load * 100.0), "%");
    push(&mut c, "ENG", "TPS", f0(v.throttle * 100.0), "%");
    push(&mut c, "ENG", "MAF", f1(v.maf_gps), "g/s");
    push(&mut c, "ENG", "ECT", f0(v.coolant_c), "C");
    push(&mut c, "ENG", "OIL", f0(v.oil_temp_c), "C");
    push(&mut c, "ENG", "IAT", f0(v.iat_c), "C");
    // FUEL / ELEC
    push(&mut c, "FUEL", "FUEL", f0(v.fuel * 100.0), "%");
    push(&mut c, "FUEL", "FP", f0(v.fuel_pressure_kpa), "kPa");
    push(&mut c, "FUEL", "BATT", f1(v.battery_v), "V");
    push(&mut c, "ELEC", "BATT", f1(v.battery_v), "V");
    push(&mut c, "ELEC", "LOAD", f0(v.load * 100.0), "%");
    push(&mut c, "ELEC", "MAF", f1(v.maf_gps), "g/s");
    // FLUID
    push(&mut c, "FLUID", "OIL", f0(v.oil_temp_c), "C");
    push(&mut c, "FLUID", "ECT", f0(v.coolant_c), "C");
    push(&mut c, "FLUID", "TFT", f0(v.trans_temp_c), "C");
    push(&mut c, "FLUID", "IAT", f0(v.iat_c), "C");
    push(&mut c, "FLUID", "EGT", f0(v.exhaust_temp_c), "C");
    push(&mut c, "FLUID", "AAT", f0(v.temp_out_c), "C");
    push(&mut c, "FLUID", "CAB", f0(v.temp_in_c), "C");
    // DRV
    push(
        &mut c,
        "DRV",
        "SPD",
        f0(v.speed_unit.from_mph(v.speed_mph)),
        v.speed_unit.name(),
    );
    push(&mut c, "DRV", "RPM", f0(v.rpm), "rpm");
    push(&mut c, "DRV", "TPS", f0(v.throttle * 100.0), "%");
    push(&mut c, "DRV", "GEAR", v.gear.label().into(), "");
    push(&mut c, "DRV", "GNUM", format!("{}", v.gear_num), "");
    push(&mut c, "DRV", "4WD", v.drive.label().into(), "");
    // CHAS
    push(&mut c, "CHAS", "FL", f1(v.tire_fl.pressure), "psi");
    push(&mut c, "CHAS", "FR", f1(v.tire_fr.pressure), "psi");
    push(&mut c, "CHAS", "RL", f1(v.tire_rl.pressure), "psi");
    push(&mut c, "CHAS", "RR", f1(v.tire_rr.pressure), "psi");
    push(&mut c, "CHAS", "WSFL", f1(v.wheel_fl_kph), "km/h");
    push(&mut c, "CHAS", "WSFR", f1(v.wheel_fr_kph), "km/h");
    push(&mut c, "CHAS", "WSRL", f1(v.wheel_rl_kph), "km/h");
    push(&mut c, "CHAS", "WSRR", f1(v.wheel_rr_kph), "km/h");
    push(
        &mut c,
        "CHAS",
        "BRK",
        if v.brake_pedal { "ON" } else { "OFF" }.into(),
        "",
    );
    push(
        &mut c,
        "CHAS",
        "PARK",
        if v.park_brake { "ON" } else { "OFF" }.into(),
        "",
    );
    push(&mut c, "CHAS", "STR", f1(v.steer_deg), "deg");
    // BODY
    push(
        &mut c,
        "BODY",
        "DFL",
        if v.door_fl { "CL" } else { "OP" }.into(),
        "",
    );
    push(
        &mut c,
        "BODY",
        "DFR",
        if v.door_fr { "CL" } else { "OP" }.into(),
        "",
    );
    push(
        &mut c,
        "BODY",
        "DRL",
        if v.door_rl { "CL" } else { "OP" }.into(),
        "",
    );
    push(
        &mut c,
        "BODY",
        "DRR",
        if v.door_rr { "CL" } else { "OP" }.into(),
        "",
    );
    // CLIM
    push(&mut c, "CLIM", "CAB", f0(v.temp_in_c), "C");
    push(&mut c, "CLIM", "SET", f0(v.hvac_set_c), "C");
    push(&mut c, "CLIM", "FAN", f0(v.hvac_fan * 100.0), "%");
    // SA
    push(&mut c, "SA", "PITCH", f1(v.pitch_deg), "deg");
    push(&mut c, "SA", "ROLL", f1(v.roll_deg), "deg");
    push(&mut c, "SA", "HDG", f1(v.heading_deg), "deg");
    // OWN
    push(
        &mut c,
        "OWN",
        "VIN",
        if v.vin.is_empty() {
            "—".into()
        } else {
            v.vin.clone()
        },
        "",
    );
    push(&mut c, "BIT", "DTC", format!("{}", v.dtc_count), "");
    // LINK / Bluetooth
    push(&mut c, "LINK", "STATE", v.bus_state.clone(), "");
    push(&mut c, "LINK", "KIND", v.bus_kind.clone(), "");
    push(
        &mut c,
        "LINK",
        "ADDR",
        if v.bus_addr.is_empty() {
            "—".into()
        } else {
            v.bus_addr.clone()
        },
        "",
    );
    push(&mut c, "LINK", "CH", v.bus_channel.clone(), "");
    push(
        &mut c,
        "LINK",
        "ADPT",
        if v.bus_adapter.is_empty() {
            "—".into()
        } else {
            v.bus_adapter.clone()
        },
        "",
    );
    push(
        &mut c,
        "LINK",
        "PROT",
        if v.bus_proto.is_empty() {
            "—".into()
        } else {
            v.bus_proto.clone()
        },
        "",
    );
    push(&mut c, "LINK", "TICK", format!("{}", v.bus_ticks), "");
    let _ = (GearSelect::Park, DriveMode::TwoHigh);
    c
}

pub fn channels_in_group(v: &VehicleSnapshot, group: &str) -> Vec<Channel> {
    all_channels(v)
        .into_iter()
        .filter(|ch| ch.group == group)
        .collect()
}
