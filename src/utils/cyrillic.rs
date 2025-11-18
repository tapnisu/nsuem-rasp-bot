pub trait ToCyrillic {
    fn to_cyrillic(&self) -> Self;
}

impl ToCyrillic for String {
    fn to_cyrillic(&self) -> String {
        self.chars()
            .map(|c| match c {
                'A' | 'a' => 'А',
                'B' | 'b' => 'В',
                'C' | 'c' => 'С',
                'E' | 'e' => 'Е',
                'H' | 'h' => 'Н',
                'K' | 'k' => 'К',
                'M' | 'm' => 'М',
                'O' | 'o' => 'О',
                'P' | 'p' => 'Р',
                'T' | 't' => 'Т',
                'X' | 'x' => 'Х',
                'Y' | 'y' => 'У',
                other => other,
            })
            .collect()
    }
}
