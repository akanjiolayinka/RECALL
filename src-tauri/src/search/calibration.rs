//! Calibration of meaning-based search with the real embedding model.
//!
//! A labelled set of synthetic documents and searches: which documents each
//! search should find, and searches that should find nothing. Most searches
//! deliberately share no words with the document they should find, because
//! that's the case only meaning-based search can handle. The test prints the
//! similarity of every search/document pair and the precision and recall of
//! each candidate threshold, to choose the thresholds in `semantic.rs`.
//!
//! Run: `cargo test calibration -- --ignored --nocapture` (needs the model).

use crate::embeddings::bge::{BgeSmall, MODEL_SUBDIR};
use crate::embeddings::{similarity, Embedder};
use crate::test_support::real_models_dir;

const DOCUMENTS: &[&str] = &[
    // 0
    "Access Bank account statement for March 2026. Opening balance NGN 312,450. Transfers out: rent NGN 150,000; school fees NGN 85,000. Closing balance NGN 61,920.",
    // 1
    "Lagos University Teaching Hospital. Your appointment with the cardiology clinic is on 14 July 2026 at 9:30am. Please bring your previous ECG results and arrive fasting.",
    // 2
    "Booking confirmation PNR K7Q2LM. Lagos (LOS) to Nairobi (NBO), Kenya Airways KQ533, departing 3 August 2026 at 06:15. One checked bag of 23 kg included.",
    // 3
    "Your comprehensive motor insurance policy for the Toyota Corolla (reg. KJA-482-AB) renews on 1 September 2026. Annual premium: NGN 96,000.",
    // 4
    "Greenfield Primary School, first term 2026/2027. Tuition NGN 180,000, uniform NGN 22,000, books NGN 15,500. Payment due before resumption on 8 September.",
    // 5
    "Blend tomatoes, red peppers and onions. Fry the paste in oil with curry and thyme, add stock and washed rice, then cover and cook on low heat for 30 minutes.",
    // 6
    "FitZone Ikeja membership: 12 months, access to the weights room, swimming pool and evening classes. Cancel with 30 days' notice.",
    // 7
    "We are pleased to offer you the position of Data Analyst at Brightpath Ltd with an annual salary of NGN 9,600,000, starting 1 November 2026.",
    // 8
    "Ikeja Electric prepaid token purchase: 120.5 kWh for NGN 25,000, meter number 0412 7789 551.",
    // 9
    "Together with their families, Ada and Emeka invite you to celebrate their marriage on Saturday 12 December 2026 at the Civic Centre, Victoria Island. Reception follows.",
    // 10
    "Warranty card: Samsung Galaxy A55, serial R58T21. Covered for 24 months against manufacturing defects. Screen damage and water damage are not covered.",
    // 11
    "Nigeria Immigration Service: to renew your passport, book an appointment online, pay the fee, and bring your old passport and national identity number (NIN) slip.",
    // 12
    "Dr. Okafor. Amoxicillin 500 mg, one capsule three times daily for 7 days. Paracetamol as needed for fever. Review in one week.",
    // 13
    "Payslip, September 2026. Gross pay NGN 800,000. Tax (PAYE) NGN 112,000. Pension NGN 64,000. Net pay NGN 624,000.",
    // 14
    "Tobi turns 7 on 20 June. Book the bouncy castle, order a chocolate cake with blue icing, and send invitations to his classmates.",
    // 15
    "Either party must give sixty days' written notice before moving out. The deposit is returned within thirty days after the keys are handed back.",
    // 16
    "The estimated project budget is NGN 2,500,000 in total. Raised beds and soil: NGN 1,100,000. Rainwater tank and pipes: NGN 650,000.",
    // 17
    "SOUNDWAVE ELECTRONICS 14 Market Road, Ikeja Wireless headphones NGN 45,000 Carrying case NGN 5,000 TOTAL NGN 50,000 Paid by card.",
    // 18
    "Total marketing spend for the quarter is capped at NGN 4,000,000, split between radio adverts, social media and two community events.",
    // 19
    "Shopping list: tomatoes, peppers, garden gloves, watering can.",
    // 20
    "Garden committee meeting. We agreed to order the rainwater tank next week. Ngozi will ask the ward office about the development grant.",
    // 21
    "Curriculum vitae. Chioma Nwosu. Experience: 4 years as a secondary school mathematics teacher. Skills: Excel, lesson planning, public speaking.",
    // 22
    "Service at Coscharis Motors: engine oil and filter changed, brake pads replaced, tyres rotated. Next service due at 45,000 km.",
    // 23
    "Invoice from QuickFix Plumbing: repaired leaking kitchen sink pipe and replaced the bathroom tap. Labour and parts NGN 18,500.",
];

/// (search, documents it should find). An empty list: it should find nothing.
const SEARCHES: &[(&str, &[usize])] = &[
    (
        "funds remaining after my outgoing payments last month",
        &[0],
    ),
    ("when is my heart doctor visit", &[1]),
    ("plane ticket to East Africa", &[2]),
    ("how much do I pay yearly to cover my vehicle", &[3]),
    ("cost of my child's schooling for the coming semester", &[4]),
    ("instructions for preparing a spicy West African meal", &[5]),
    ("fitness club contract", &[6]),
    ("employment letter with my pay", &[7, 13]),
    ("how much power did I buy", &[8]),
    ("when is the couple's big day", &[9]),
    ("is my mobile repair free if it breaks", &[10]),
    ("what do I need to get a new travel booklet", &[11]),
    ("antibiotics the doctor gave me", &[12]),
    ("deductions from my monthly earnings", &[13]),
    ("my son's celebration plans", &[14]),
    ("how early must I tell the landlord I'm leaving", &[15]),
    ("what is the price of the allotment", &[16]),
    ("headset purchase", &[17]),
    ("promotion expenses", &[18]),
    ("groceries to buy", &[19]),
    ("who is following up on funding", &[20]),
    ("job application profile of an educator", &[21]),
    ("car maintenance history", &[22]),
    ("who fixed the dripping faucet", &[23]),
    // Nothing in the documents is about these.
    ("quantum physics lecture", &[]),
    ("football match results", &[]),
    ("how to train a puppy", &[]),
    ("volcano eruption facts", &[]),
    ("learning to play the guitar", &[]),
    ("stock market investing tips", &[]),
    ("knitting a scarf pattern", &[]),
    ("history of the Roman empire", &[]),
    ("solar system planets", &[]),
    ("chess opening strategies", &[]),
];

#[test]
#[ignore = "needs the BGE model files"]
fn calibration() {
    let model = BgeSmall::load(&real_models_dir().join(MODEL_SUBDIR)).expect("BGE model files");
    let documents = model.embed_passages(DOCUMENTS).unwrap();

    // (similarity, relevant) for every search/document pair.
    let mut pairs: Vec<(f32, bool)> = Vec::new();
    for (search, relevant) in SEARCHES {
        let query = model.embed_query(search).unwrap();
        let mut scored: Vec<(usize, f32)> = documents
            .iter()
            .enumerate()
            .map(|(i, d)| (i, similarity(&query, d)))
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        let shown: Vec<String> = scored
            .iter()
            .take(4)
            .map(|(i, s)| format!("{s:.3}{}#{i}", if relevant.contains(i) { "*" } else { " " }))
            .collect();
        println!("{search:55} {}", shown.join("  "));
        pairs.extend(scored.iter().map(|&(i, s)| (s, relevant.contains(&i))));
    }

    let relevant_total = pairs.iter().filter(|p| p.1).count();
    println!("\nthreshold  found/relevant  wrong  precision  recall  F1");
    for step in 0..=30 {
        let threshold = 0.45 + step as f32 * 0.01;
        let found = pairs.iter().filter(|p| p.1 && p.0 >= threshold).count();
        let wrong = pairs.iter().filter(|p| !p.1 && p.0 >= threshold).count();
        let precision = found as f32 / (found + wrong).max(1) as f32;
        let recall = found as f32 / relevant_total as f32;
        let f1 = 2.0 * precision * recall / (precision + recall).max(f32::EPSILON);
        println!(
            "{threshold:.2}       {found:2}/{relevant_total}           {wrong:3}    {precision:.2}       {recall:.2}    {f1:.2}"
        );
    }
    let mut relevant: Vec<f32> = pairs.iter().filter(|p| p.1).map(|p| p.0).collect();
    relevant.sort_by(f32::total_cmp);
    println!("\nrelevant similarities, sorted: {relevant:.3?}");
}
