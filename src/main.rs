use scraper::{Html, Selector};

#[tokio::main]
async fn main() {
    let req = reqwest::get("https://rasps.nsuem.ru/group/%D0%98%D0%A1502/2")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let document = Html::parse_document(&req);
    let row_selector = Selector::parse("table.table tr").unwrap();
    let day_selector = Selector::parse("td.day-header").unwrap();
    let time_selector = Selector::parse("td .extend_time").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let info_selector = Selector::parse(".mainScheduleInfo").unwrap();
    let teacher_selector = Selector::parse(".Teacher a").unwrap();
    let type_selector = Selector::parse(".small.text-muted").unwrap();

    let week_cell = match document
        .select(&Selector::parse("td#blink").unwrap())
        .next()
        .map(|cell| cell.text().collect::<String>().trim().to_string())
        .as_deref()
    {
        Some("Вторая неделя") => 3,
        Some("Первая неделя") | None | Some(_) => 2,
    };

    let mut output: Vec<String> = vec![];

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&td_selector).collect();

        if cells.len() < week_cell + 1 {
            continue;
        }

        if let Some(day_cell) = row.select(&day_selector).next() {
            let current_day = day_cell.text().collect::<String>().trim().to_string();
            output.push(format!("{}:", current_day));
        }

        let time = row
            .select(&time_selector)
            .next()
            .map(|t| {
                t.text()
                    .collect::<String>()
                    .trim()
                    .replace("--", "-")
                    .to_string()
            })
            .unwrap_or_default();

        let cell = &cells[week_cell];
        if cell.select(&info_selector).next().is_none() {
            continue;
        }

        let subject = cell
            .select(&info_selector)
            .next()
            .map(|c| {
                c.text()
                    .collect::<String>()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();

        let lesson_type = cell
            .select(&type_selector)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let teacher = cell
            .select(&teacher_selector)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        output.push(format!(
            "{} | {} | {} | {}",
            time, subject, teacher, lesson_type
        ));
    }

    println!("{}", output.join("\n"));
}
