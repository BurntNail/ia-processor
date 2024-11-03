use std::collections::{HashMap, HashSet};
use std::fs::{File};
use std::path::PathBuf;
use chrono::{NaiveDate};
use clap::Parser;
use color_eyre::eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Deserialize, Debug, Clone)]
struct InputPerson {
    #[allow(dead_code)]
    award_unit: String,
    first_name: String,
    middle_name: String,
    last_name: String,
    #[allow(dead_code)]
    award_level: String,
    #[allow(dead_code)]
    sub_activity: String,
    #[allow(dead_code)]
    aim: String,
    completed: f64,
    #[allow(dead_code)]
    first_log_date: String,
    #[allow(dead_code)]
    assessor_name: String,
    #[allow(dead_code)]
    assessor_email: String,
    #[allow(dead_code)]
    pid: u32,
    last_log: String,
    #[allow(dead_code)]
    gender: String
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct ToBeEmailed {
    first_name: String,
    middle_name: String,
    last_name: String
}

impl From<&InputPerson> for ToBeEmailed {
    fn from(value: &InputPerson) -> Self {
        Self {
            first_name: value.first_name.clone(),
            middle_name: value.middle_name.clone(),
            last_name: value.last_name.clone()
        }
    }
}

#[derive(Parser)]
struct Args {
    input_file: PathBuf,
    first_name_filter: String,
    output_for_time: PathBuf,
    output_for_emails: PathBuf
}


fn setup() {
    color_eyre::install().expect("unable to install color-eyre");

    if cfg!(debug_assertions) {
        for key in &["RUST_SPANTRACE", "RUST_LIB_BACKTRACE", "RUST_BACKTRACE"] {
            if std::env::var(key).is_err() {
                std::env::set_var(key, "full");
            }
        }
    }
}

fn main() -> color_eyre::Result<()> {
    setup();
    let Args { input_file, first_name_filter, output_for_time, output_for_emails } = Args::parse();

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .from_reader(File::open(input_file)?);
    let mut records: Vec<InputPerson> = rdr.deserialize().collect::<Result<_, _>>()?;
    records.retain(|rec| {
        rec.first_name.contains(&first_name_filter)
    });

    sort_out_two_weeks(records.clone().into_iter().map(|x| (x.pid, x)).collect(), output_for_emails)?;
    output_time(records, output_for_time)?;


    Ok(())
}

fn output_time (mut records: Vec<InputPerson>, output: PathBuf) -> color_eyre::Result<()> {
    let mut to_be_output = HashMap::new();
    for rec in records {
        let key = (rec.last_name, rec.first_name);
        *to_be_output.entry(key).or_default() += rec.completed;
    }

    let mut to_be_output: Vec<(_, f64)> = to_be_output.into_iter().collect();
    to_be_output.sort_by_key(|((l, _f), _)| l.clone());
    to_be_output.sort_by_key(|((_l, f), _)| f.clone());


    let mut file = File::create(output)?;
    for person in to_be_output {
        writeln!(file, "{}", person.1)?;
    }

    Ok(())
}


fn sort_out_two_weeks(by_pid: HashMap<u32, InputPerson>, output: PathBuf) -> color_eyre::Result<()> {
    let mut no_need_to_email = HashSet::new();
    for (_, person) in &by_pid {
        let needs_to_be_emailed = if person.last_log.is_empty() {
            true
        } else {
            let (year, month, date) = {
                let last_log_str = &person.last_log;
                let year = &last_log_str[0..4];
                let month = &last_log_str[5..7];
                let day = &last_log_str[8..10];

                (year.parse()?, month.parse()?, day.parse()?)
            };

            let last_log = NaiveDate::from_ymd_opt(year, month, date).context("trying to get date from string")?;
            let today = chrono::offset::Local::now().date_naive();
            let delta = today - last_log;
            delta.num_days() >= 14
        };

        if !needs_to_be_emailed {
            no_need_to_email.insert(person.pid);
        }
    }

    let to_be_emailed: HashSet<ToBeEmailed> = by_pid.iter().filter_map(|(pid, person)| {
        (!no_need_to_email.contains(pid)).then(|| person.into())
    }).collect();
    let mut to_be_emailed: Vec<ToBeEmailed> = to_be_emailed.into_iter().collect();
    to_be_emailed.sort_by_key(|x| x.last_name.clone());
    to_be_emailed.sort_by_key(|x| x.first_name.clone());


    let mut file = File::create(output)?;
    for ToBeEmailed { first_name, middle_name, last_name } in to_be_emailed {
        writeln!(file, "{first_name} {middle_name} {last_name}")?;
    }

    Ok(())
}