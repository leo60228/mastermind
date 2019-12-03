use arrayvec::ArrayVec;
use fn_memo::{FnMemo, sync, recur_fn::direct};
use std::collections::BTreeSet;
use std::convert::{TryFrom, TryInto};
use std::io;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Digit {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Zero,
}

impl From<Digit> for u8 {
    fn from(d: Digit) -> Self {
        use Digit::*;

        match d {
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
            Six => 6,
            Seven => 7,
            Eight => 8,
            Nine => 9,
            Zero => 0,
        }
    }
}

impl TryFrom<u8> for Digit {
    type Error = ();

    fn try_from(d: u8) -> Result<Self, Self::Error> {
        use Digit::*;

        Ok(match d {
            1 => One,
            2 => Two,
            3 => Three,
            4 => Four,
            5 => Five,
            6 => Six,
            7 => Seven,
            8 => Eight,
            9 => Nine,
            0 => Zero,
            _ => Err(())?,
        })
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Code([Digit; 6]);

impl From<Code> for u32 {
    fn from(c: Code) -> Self {
        c.0.iter()
            .rev()
            .enumerate()
            .map(|(i, d)| u32::from(u8::from(*d)) * 10_u32.pow(i.try_into().unwrap()))
            .sum()
    }
}

impl From<u32> for Code {
    fn from(c: u32) -> Self {
        let vec: ArrayVec<[_; 6]> = (0..6)
            .rev()
            .map(|i| Digit::try_from(((c / 10_u32.pow(i)) % 10) as u8).unwrap())
            .collect();
        Self(vec.into_inner().unwrap())
    }
}

pub fn check(code: Code, guess: Code) -> (u8, u8) {
    guess
        .0
        .iter()
        .enumerate()
        .fold((0, 0), |(good, miss), (i, g)| {
            if *g == code.0[i] {
                (good + 1, miss)
            } else if (code
                .0
                .iter()
                .enumerate()
                .filter(|(i, c)| g == *c && guess.0[*i] != code.0[*i])
                .take(1)
                .count() as isize)
                - (guess
                    .0
                    .iter()
                    .take(i)
                    .filter(|g2| *g2 == g && code.0.iter().any(|c| *g == *c))
                    .count() as isize)
                > 0
            {
                (good, miss + 1)
            } else {
                (good, miss)
            }
        })
}

pub fn break_code(mut good: impl FnMut(Code) -> (u8, u8)) -> Option<Code> {
    let checker = sync::chashmap::memoize(direct(|(c, g)| check(c, g)));
    let mut guesses: BTreeSet<_> = (0..=999999).map(Code::from).collect();
    while guesses.len() > 1 {
        let guess = *guesses
                .iter()
                //.min_by_key(|g| guesses.iter().max_by_key(|g2| checker.call((**g, **g2))))
                .next()
                .unwrap();
        let resp = good(guess);
        guesses = guesses
            .into_par_iter()
            .filter(|g| checker.call((*g, guess)) == resp)
            .collect();
    }
    guesses.iter().next().map(|&x| x)
}

fn main() {
    let mut response_buf = String::new();
    let code = break_code(|guess| {
        println!("{:06}", u32::from(guess));
        response_buf.clear();
        io::stdin().read_line(&mut response_buf).unwrap();
        (
            response_buf.chars().nth(0).unwrap().to_digit(10).unwrap() as u8,
            response_buf.chars().nth(1).unwrap().to_digit(10).unwrap() as u8,
        )
    });

    println!("done {:06}", u32::from(code.expect("Couldn't find code!")));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn code() {
        use Digit::*;

        let code = Code([One, Two, Three, Four, Zero, Six]);
        assert_eq!(u32::from(code), 123406);
        assert_eq!(Code::from(123406), code);
    }

    #[test]
    fn check() {
        use super::check;
        use Digit::*;

        let code = Code([One, Two, Three, Four, Zero, Six]);
        assert_eq!(check(code, code), (6, 0));

        let rev = Code([Six, Zero, Four, Three, Two, One]);
        assert_eq!(check(code, rev), (0, 6));

        let bad = Code([Nine, Nine, Nine, Nine, Nine, Nine]);
        assert_eq!(check(code, bad), (0, 0));

        let swapped = Code([One, Two, Three, Four, Six, Zero]);
        assert_eq!(check(code, swapped), (4, 2));

        let partial = Code([One, Two, Three, Four, Nine, Nine]);
        assert_eq!(check(code, partial), (4, 0));

        let dupe = Code([One, Two, Three, Four, Zero, Zero]);
        assert_eq!(check(code, dupe), (5, 0));

        let dupe_code = Code([One, One, One, Two, Two, Two]);
        let dupe_miss = Code([One, One, One, One, Two, Two]);
        assert_eq!(check(dupe_code, dupe_miss), (5, 0));

        let dupe_double_code = Code([One, Two, Nine, Nine, Nine, Nine]);
        let dupe_double_miss = Code([One, One, Two, Two, Two, Two]);
        assert_eq!(check(dupe_double_code, dupe_double_miss), (1, 1));
        assert_eq!(check(dupe_double_miss, dupe_double_code), (1, 1));
    }

    #[test]
    fn break_code() {
        use super::*;

        fn test(code: u32) {
            let code = dbg!(code).into();
            assert_eq!(
                break_code(|guess| {
                    let resp = check(code, guess);
                    println!("{} {} = {:?}", u32::from(code), u32::from(guess), resp);
                    resp
                }),
                Some(code)
            );
        }

        test(123406);
        test(111111);
        test(123456);
        test(81220);
        test(1);
    }
}
