use crate::api::activities::Activity;
use colored::Colorize;

pub fn print_activities(activities: &[Activity]) {
    if activities.is_empty() {
        println!("{}", "  (no recent activity)".dimmed());
        return;
    }
    for a in activities {
        print_activity(a);
    }
}

pub fn print_activity(activity: &Activity) {
    let ts = format_ts(&activity.create_time);

    if let Some(msg) = &activity.message {
        let (prefix, text) = if msg.author.to_uppercase() == "USER" {
            (format!("{} you  ", "▶".blue()), msg.text.as_str().blue().to_string())
        } else {
            (format!("{} jules", "◀".magenta()), msg.text.as_str().normal().to_string())
        };
        println!("{} {}\n  {}\n", ts.dimmed(), prefix, text);
    } else if let Some(plan) = &activity.plan {
        println!(
            "{} {} {}\n  {}\n",
            ts.dimmed(),
            "⚙ plan".yellow(),
            format!("[{}]", plan.status).dimmed(),
            plan.description.as_str().dimmed(),
        );
    } else if let Some(push) = &activity.github_push {
        println!(
            "{} {} branch={} sha={}\n",
            ts.dimmed(),
            "↑ github push".green().bold(),
            push.branch.green(),
            push.commit_sha.dimmed(),
        );
    }
}

fn format_ts(iso: &str) -> String {
    if iso.len() >= 16 {
        iso[11..16].to_string()
    } else if iso.is_empty() {
        "--:--".to_string()
    } else {
        iso.to_string()
    }
}
