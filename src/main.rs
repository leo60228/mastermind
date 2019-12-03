use arrayvec::ArrayVec;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::{TryFrom, TryInto};
use std::io;
use rand::prelude::*;

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

pub fn possibilities<'a>(guess: Code, guesses: &'a BTreeMap<Code, (u8, u8)>) -> impl ParallelIterator<Item = (u8, u8)> + 'a {
    guesses
        .par_iter()
        .flat_map(move |(g2, (good, miss))| {
            let good = 0..=guess
                .0
                .iter()
                .zip(g2.0.iter())
                .filter(|(x, y)| x == y)
                .count()
                .min((*good).into());
            let miss = 0..=guess
                .0
                .iter()
                .filter(|x| g2.0.iter().any(|y| *x == y))
                .count()
                .min((*miss).into());
            good.cartesian_product(miss).par_bridge()
        })
        .map(|(x, y)| (x as u8, y as u8))
}

pub fn break_code(mut good: impl FnMut(Code) -> (u8, u8)) -> Option<Code> {
    let mut guesses: BTreeSet<_> = (0..=999999).map(Code::from).collect();
    let mut prev = BTreeMap::new();
    while guesses.len() > 1 {
        let guess: Code = if prev.len() == 0 {
            111222_u32.into()
        } else {
            *guesses
                .par_iter()
                .filter(|_| thread_rng().gen_range(0, 500) == 0) // probabilistic: 1 million guesses is too many
                .max_by_key(|&&g| {
                    guesses.len()
                        - possibilities(g, &prev)
                            .map(|p| guesses.par_iter().filter(|&&g2| check(g2, g) != p).count())
                            .min()
                            .unwrap()
                })
                .unwrap_or_else(|| guesses.iter().next().unwrap())
        };
        let resp = good(guess);
        prev.insert(guess, resp);
        guesses = guesses
            .into_par_iter()
            .filter(|g| check(*g, guess) == resp)
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
