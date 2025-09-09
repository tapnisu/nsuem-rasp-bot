use std::fmt;

use scraper::{Html, Selector};

#[derive(Debug, Clone)]
pub struct Schedule {
    pub weeks: Vec<Week>,
    pub current_week: usize,
}

#[derive(Debug, Clone)]
pub struct Week {
    pub days: Vec<Option<Day>>,
}

#[derive(Debug, Clone)]
pub struct Day {
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone)]
pub struct Lesson {
    pub time: String,
    pub time_extended: String,
    pub subject: String,
    pub lesson_type: String,
    pub teacher: String,
}

impl fmt::Display for Lesson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<code>{}</code> {} ({}) | {}",
            self.time_extended, self.subject, self.lesson_type, self.teacher
        )
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for lesson in &self.lessons {
            writeln!(f, "{}", lesson)?;
        }

        Ok(())
    }
}

impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, day) in self.days.iter().enumerate() {
            let day_name = match i {
                0 => "Понедельник",
                1 => "Вторник",
                2 => "Среда",
                3 => "Четверг",
                4 => "Пятница",
                5 => "Суббота",
                6 => "Воскресенье:",
                _ => unreachable!(),
            };

            if let Some(day) = day {
                writeln!(f, "{}:", day_name)?;
                writeln!(f, "{}", day)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Текущая неделя: {}", self.current_week)?;

        for (i, week) in self.weeks.iter().enumerate() {
            writeln!(f, "Неделя {}:", i + 1)?;
            write!(f, "{}", week)?;
        }
        Ok(())
    }
}

impl Schedule {
    pub async fn new(group_name: &str) -> Schedule {
        let url = format!("https://rasps.nsuem.ru/group/{}", group_name);
        let html = reqwest::get(&url).await.unwrap().text().await.unwrap();

        let document = Html::parse_document(&html);
        let row_selector = Selector::parse("table.table tr").unwrap();
        let day_selector = Selector::parse("td.day-header").unwrap();
        let time_selector = Selector::parse("td .time").unwrap();
        let extended_time_selector = Selector::parse("td .extend_time").unwrap();
        let td_selector = Selector::parse("td").unwrap();
        let info_selector = Selector::parse(".mainScheduleInfo").unwrap();
        let teacher_selector = Selector::parse(".Teacher a").unwrap();
        let type_selector = Selector::parse(".small.text-muted").unwrap();

        let current_week = match document
            .select(&Selector::parse("td#blink").unwrap())
            .next()
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .as_deref()
        {
            Some("Вторая неделя") => 2,
            Some("Первая неделя") | None | Some(_) => 1,
        };

        let mut weeks = vec![
            Week {
                days: vec![None; 7],
            },
            Week {
                days: vec![None; 7],
            },
        ];

        let mut current_day_index = 0;

        for row in document.select(&row_selector) {
            if let Some(day_cell) = row.select(&day_selector).next() {
                let day_name = day_cell.text().collect::<String>().trim().to_string();
                current_day_index = match day_name.as_str() {
                    "пн" => 0,
                    "вт" => 1,
                    "ср" => 2,
                    "чт" => 3,
                    "пт" => 4,
                    "сб" => 5,
                    "вс" => 6,
                    _ => unreachable!(),
                };
            }

            let cells: Vec<_> = row.select(&td_selector).collect();
            if cells.len() < 4 {
                continue;
            }

            // quick reminder: 0..2 includes [0, 1]
            for week_index in 0..2 {
                let cell = &cells[2 + week_index];

                if cell.select(&info_selector).next().is_none() {
                    continue;
                }

                let time = row
                    .select(&time_selector)
                    .next()
                    .map(|t| t.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                let extended_time = row
                    .select(&extended_time_selector)
                    .next()
                    .map(|t| {
                        t.text()
                            .collect::<String>()
                            .trim()
                            .replace("--", "-")
                            .to_string()
                    })
                    .unwrap_or_default();

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

                let lesson = Lesson {
                    time,
                    time_extended: extended_time,
                    subject,
                    lesson_type,
                    teacher,
                };

                if let Some(day) = &mut weeks[week_index].days[current_day_index] {
                    day.lessons.push(lesson);
                } else {
                    weeks[week_index].days[current_day_index] = Some(Day {
                        lessons: vec![lesson],
                    });
                }
            }
        }

        Schedule {
            weeks,
            current_week,
        }
    }
}
