use fontnorm::core::family::{FamilyGrouping, FamilyMember, family_key};
use fontnorm::core::model::{RibbiSlot, Weight, Width};

fn member(weight: Weight, italic: bool, width: Width) -> FamilyMember {
    FamilyMember {
        file_index: 0,
        sort_key: String::new(),
        weight,
        italic,
        width,
    }
}

#[test]
fn four_ribbi_members_do_not_exceed() {
    let members = vec![
        member(Weight::REGULAR, false, Width::NORMAL),
        member(Weight::BOLD, false, Width::NORMAL),
        member(Weight::REGULAR, true, Width::NORMAL),
        member(Weight::BOLD, true, Width::NORMAL),
    ];
    let g = FamilyGrouping::new("Yrsa".into(), members);
    assert!(!g.exceeds_ribbi);
}

#[test]
fn extra_weight_exceeds_ribbi() {
    let members = vec![
        member(Weight::REGULAR, false, Width::NORMAL),
        member(Weight::SEMI_BOLD, false, Width::NORMAL),
    ];
    let g = FamilyGrouping::new("Caslon".into(), members);
    assert!(g.exceeds_ribbi);
}

#[test]
fn duplicate_slot_exceeds_ribbi() {
    // N4: two Regulars in one ID1 group is illegal.
    let members = vec![
        member(Weight::REGULAR, false, Width::NORMAL),
        member(Weight::REGULAR, false, Width::NORMAL),
    ];
    let g = FamilyGrouping::new("Yrsa".into(), members);
    assert!(g.exceeds_ribbi);
}

#[test]
fn assign_ribbi_weight_stays_in_base_family() {
    let g = FamilyGrouping::new("Yrsa".into(), vec![]);
    let a = g.assign(&member(Weight::BOLD, true, Width::NORMAL));
    assert_eq!(a.ribbi_family, "Yrsa");
    assert_eq!(a.ribbi_slot, RibbiSlot::BoldItalic);
    assert_eq!(a.dup_ordinal, 0);
}

#[test]
fn assign_semibold_splits() {
    let g = FamilyGrouping::new("Adobe Caslon Pro".into(), vec![]);
    let a = g.assign(&member(Weight::SEMI_BOLD, false, Width::NORMAL));
    assert_eq!(a.ribbi_family, "Adobe Caslon Pro SemiBold");
    assert_eq!(a.ribbi_slot, RibbiSlot::Regular);
}

#[test]
fn assign_semibold_italic_splits_italic_slot() {
    let g = FamilyGrouping::new("Adobe Caslon Pro".into(), vec![]);
    let a = g.assign(&member(Weight::SEMI_BOLD, true, Width::NORMAL));
    assert_eq!(a.ribbi_family, "Adobe Caslon Pro SemiBold");
    assert_eq!(a.ribbi_slot, RibbiSlot::Italic);
}

#[test]
fn assign_condensed_splits_on_width() {
    let g = FamilyGrouping::new("Helvetica".into(), vec![]);
    let a = g.assign(&member(Weight::REGULAR, false, Width(3)));
    assert_eq!(a.ribbi_family, "Helvetica Condensed");
}

#[test]
fn assign_dup_slot_gets_ordinal() {
    // Two identical Regular faces (N4 dup-slot): the second gets dup_ordinal 1.
    let m0 = FamilyMember {
        file_index: 0,
        sort_key: "Yrsa-Regular".into(),
        weight: Weight::REGULAR,
        italic: false,
        width: Width::NORMAL,
    };
    let m1 = FamilyMember {
        file_index: 1,
        sort_key: "Yrsa-Regular-alt".into(),
        weight: Weight::REGULAR,
        italic: false,
        width: Width::NORMAL,
    };
    let g = FamilyGrouping::new("Yrsa".into(), vec![m0.clone(), m1.clone()]);
    assert_eq!(g.assign(&m0).dup_ordinal, 0);
    assert_eq!(g.assign(&m1).dup_ordinal, 1);
}

#[test]
fn assign_dup_ordinal_is_keyed_on_sort_key_not_file_index() {
    // D2 regression: the ordinal must follow the STABLE sort_key, not processing order.
    // Here file_index order is REVERSED relative to sort_key order; the ordinal must still
    // follow sort_key so the same font gets the same suffix on every run.
    let early_key_late_index = FamilyMember {
        file_index: 9,
        sort_key: "AAA-Regular".into(),
        weight: Weight::EXTRA_BOLD,
        italic: false,
        width: Width::NORMAL,
    };
    let late_key_early_index = FamilyMember {
        file_index: 0,
        sort_key: "ZZZ-Regular".into(),
        weight: Weight::EXTRA_BOLD,
        italic: false,
        width: Width::NORMAL,
    };
    let g = FamilyGrouping::new(
        "Fam".into(),
        vec![early_key_late_index.clone(), late_key_early_index.clone()],
    );
    // "AAA" sorts first regardless of its high file_index -> ordinal 0.
    assert_eq!(g.assign(&early_key_late_index).dup_ordinal, 0);
    assert_eq!(g.assign(&late_key_early_index).dup_ordinal, 1);
}

#[test]
fn family_key_normalizes() {
    assert_eq!(family_key("  Adobe  Caslon Pro "), "adobe caslon pro");
}
