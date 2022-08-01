use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Utc, Weekday};
use chronoutil::delta::shift_months;
use dotenv::dotenv;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::error::Error;

const FRENCH_MONTHS: [&'static str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

const FRENCH_DAYS: [&'static str; 7] = [
    "Lundi",
    "Mardi",
    "Mercredi",
    "Jeudi",
    "Vendredi",
    "\x1b[2mSamedi\x1b[0m",
    "\x1b[2mDimanche\x1b[0m",
];

#[derive(Debug, Default)]
struct Task {
    custom_id: String,
    name: String,
    duration: i64,
    list_id: String,
}

#[derive(Debug, Default)]
struct BillLine {
    description: String,
    hours: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let raw_api_key = env::var("API_KEY").ok().unwrap();
    let api_key: &str = raw_api_key.as_str();
    let raw_org_id = env::var("ORG_ID").ok().unwrap();
    let org_id: &str = raw_org_id.as_str();

    match per_days(api_key, org_id).await {
        Err(e) => println!("{:?}", e),
        _ => (),
    }

    match per_months(api_key, org_id).await {
        Err(e) => println!("{:?}", e),
        _ => (),
    }

    Ok(())
}

async fn per_months(api_key: &str, org_id: &str) -> Result<(), Box<dyn Error>> {
    let now: DateTime<Utc> = Utc::now();
    let mut current_month: NaiveDateTime =
        NaiveDate::from_ymd(now.year(), now.month(), 1).and_hms(0, 0, 0);
    let mut next_month: NaiveDateTime = shift_months(current_month.clone(), 1);

    let client: Client = Client::new();

    let mut endpoint_tasks: String = "https://api.clickup.com/api/v2/team/".to_owned();
    endpoint_tasks.push_str(org_id);
    endpoint_tasks.push_str("/task?include_closed=1&custom_fields%5B%5D=%5B%7B%22field_id%22%3A%20%22e2407157-67b9-4edc-964b-19550176c7ee%22%2C%22operator%22%3A%20%22%3D%22%2C%20%22value%22%3A%20%220%22%7D%5D");

    let tasks_result: serde_json::Value = client
        .get(endpoint_tasks)
        .header(AUTHORIZATION, api_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await?
        .json()
        .await
        .unwrap();

    let mut tasks: HashMap<String, Task> = HashMap::new();
    for t in tasks_result["tasks"].as_array().unwrap().iter() {
        tasks.insert(
            t.get("id").unwrap().as_str().unwrap().to_string(),
            Task {
                custom_id: t.get("custom_id").unwrap().as_str().unwrap().to_string(),
                name: t.get("name").unwrap().as_str().unwrap().to_string(),
                duration: 0,
                list_id: t
                    .get("list")
                    .unwrap()
                    .get("id")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            },
        );
    }

    println!("\n\x1b[1;93mÀ facturer / total\x1b[0m");

    let mut total: i64 = 0;
    let mut total_billable: i64 = 0;
    for _ in 1..4 {
        let mut endpoint_tracked_time: String = "https://api.clickup.com/api/v2/team/".to_owned();
        endpoint_tracked_time.push_str(org_id);
        endpoint_tracked_time.push_str("/time_entries?start_date=");
        endpoint_tracked_time.push_str(&current_month.timestamp_millis().to_string());
        endpoint_tracked_time.push_str("&end_date=");
        endpoint_tracked_time.push_str(&next_month.timestamp_millis().to_string());

        let tracked_time_result: serde_json::Value = client
            .get(endpoint_tracked_time)
            .header(AUTHORIZATION, api_key)
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?
            .json()
            .await
            .unwrap();

        for i in tracked_time_result["data"].as_array().unwrap().iter() {
            let duration = i
                .borrow()
                .get("duration")
                .unwrap()
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap();

            let task_id = i.get("task").unwrap().get("id").unwrap().as_str().unwrap();
            if tasks.contains_key(task_id.to_string().as_str()) {
                total_billable += duration;
                tasks.get_mut(task_id).unwrap().duration += duration;
            }
            total += duration;
        }

        let mut total_hours: f64 = total as f64 / 3600000 as f64;
        total_hours = f64::trunc(total_hours * 1000.0) / 1000.0;

        let mut total_billable_hours: f64 = total_billable as f64 / 3600000 as f64;
        total_billable_hours = f64::trunc(total_billable_hours * 1000.0) / 1000.0;

        println!(
            "{} / {} {}",
            total_billable_hours,
            total_hours,
            FRENCH_MONTHS[(current_month.month() - 1) as usize],
        );

        total = 0;
        total_billable = 0;
        current_month = shift_months(current_month, -1);
        next_month = shift_months(next_month, -1);
    }

    let mut bills: HashMap<String, Vec<BillLine>> = HashMap::new();
    for (_, task) in tasks {
        if task.duration > 0 {
            if !bills.contains_key(task.list_id.as_str().borrow()) {
                bills.insert(task.list_id.as_str().borrow().to_string(), Vec::new());
            }

            let mut description: String = task.custom_id;
            description.push_str(" - ");
            description.push_str(task.name.as_str().borrow());

            bills
                .get_mut(task.list_id.as_str())
                .unwrap()
                .push(BillLine {
                    description,
                    hours: task.duration as f64 / 3600000 as f64,
                })
        }
    }

    for (list_id, bill_lines) in bills {
        let mut endpoint_list: String = "https://api.clickup.com/api/v2/list/".to_owned();
        endpoint_list.push_str(list_id.as_str());

        let list_result: serde_json::Value = client
            .get(endpoint_list)
            .header(AUTHORIZATION, api_key)
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?
            .json()
            .await
            .unwrap();

        let mut last_month: NaiveDate = NaiveDate::from_ymd(now.year(), now.month(), 1);
        last_month = shift_months(last_month, -1);

        println!(
            "\n\x1b[1;93m{} - {} {}\x1b[0m",
            &list_result["name"].as_str().unwrap(),
            FRENCH_MONTHS[(last_month.month() - 1) as usize],
            last_month.year(),
        );

        for bill_line in bill_lines {
            println!(
                "{} \x1b[1;31m{}\x1b[0m",
                bill_line.description, bill_line.hours
            );
        }
    }

    Ok(())
}

async fn per_days(api_key: &str, org_id: &str) -> Result<(), Box<dyn Error>> {
    let now: NaiveDateTime = Utc::now().naive_utc();
    let last_week: u32 = (now - Duration::weeks(1)).iso_week().week();
    let current_year: i32 = now.year();
    let last_monday: NaiveDateTime =
        NaiveDate::from_isoywd(current_year, last_week, Weekday::Mon).and_hms(0, 0, 0);

    let mut current_day: NaiveDateTime = last_monday.clone();
    let mut next_day: NaiveDateTime = (current_day.clone() + Duration::days(1))
        .date()
        .and_hms(0, 0, 0);

    let client = Client::new();
    let mut total: i64 = 0;

    println!("\n\x1b[1;93mTemps rentrés depuis la semaine dernière\x1b[0m");

    for _ in 1..15 {
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
            total += i
                .get("duration")
                .unwrap()
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap();
        }

        let mut total_hours: f64 = total as f64 / 3600000 as f64;
        total_hours = f64::trunc(total_hours * 1000.0) / 1000.0;
        println!(
            "\x1b[1;31m{}\x1b[0m {} {} {}",
            total_hours,
            FRENCH_DAYS[current_day.weekday() as usize],
            current_day.format("%d"),
            FRENCH_MONTHS[(current_day.month() - 1) as usize],
        );

        total = 0;
        if current_day.signed_duration_since(now).num_days() == 0 {
            break;
        }

        current_day = (current_day + Duration::days(1)).date().and_hms(0, 0, 0);
        next_day = (next_day + Duration::days(1)).date().and_hms(0, 0, 0);
    }
    Ok(())
}
