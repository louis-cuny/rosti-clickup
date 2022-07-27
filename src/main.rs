use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Utc, Weekday};
use dotenv::dotenv;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::env;
use std::error::Error;

#[allow(warnings)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let raw_api_key = env::var("API_KEY").ok().unwrap();
    let api_key: &str = raw_api_key.as_str();
    let raw_org_id = env::var("ORG_ID").ok().unwrap();
    let org_id: &str = raw_org_id.as_str();

    let last_week: u32 = (Utc::now().naive_utc() - Duration::weeks(1))
        .iso_week()
        .week();
    let current_year: i32 = chrono::offset::Local::now().year();
    let last_monday: NaiveDateTime =
        NaiveDate::from_isoywd(current_year, last_week, Weekday::Mon).and_hms(0, 0, 0);

    let mut current_day: NaiveDateTime = last_monday.clone();
    let mut next_day: NaiveDateTime = (current_day.clone() + Duration::days(1))
        .date()
        .and_hms(0, 0, 0);

    let client = reqwest::Client::new();
    let mut total: i64 = 0;

    for n in 1..15 {
        let mut endpoint: String = "https://api.clickup.com/api/v2/team/".to_owned();
        endpoint.push_str(org_id);
        endpoint.push_str("/time_entries?start_date=");
        endpoint.push_str(&current_day.timestamp_millis().to_string());
        endpoint.push_str("&end_date=");
        endpoint.push_str(&next_day.timestamp_millis().to_string());

        let res: serde_json::Value = client
            .get(endpoint)
            .header(AUTHORIZATION, api_key)
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?
            .json()
            .await
            .unwrap();

        for i in res["data"].as_array().unwrap().iter() {
            // dbg!(i.get("duration").unwrap());
            total += i
                .get("duration")
                .unwrap()
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap();
        }

        let total_hours: f64 = total as f64 / 3600000 as f64;
        println!("{} {}", total_hours, current_day.format("%A %d/%m"));

        total = 0;
        current_day = (current_day + Duration::days(1)).date().and_hms(0, 0, 0);
        next_day = (next_day + Duration::days(1)).date().and_hms(0, 0, 0);
    }

    Ok(())
}

// ROADMAP
// Temps total logué par jour Attention à minuit
// Temps par projet par mois
// Temps à facturer par projet
// Détail facture pour un projet pour le mois (implicite, on prend tout ce qui est à facturer)
