/// "A Key Length Search" [num-bigint+num-traits version]
/// Ported from matlab to Rust using phind.com
fn key_length_search_num(search_from: u32, search_to: u32) -> Option<u32> {
    use num_bigint::BigInt;
    use num_traits::identities::One;
    use num_traits::identities::Zero;

    // Constants
    let sigma: u32 = 80;
    let kappa: u32 = 256;

    // Variables
    let negl = BigInt::from(2).pow(sigma) / 2;

    // Main loop
    let mut ell: Option<u32> = None;
    for cur_ell in search_from..search_to {
        let mut badprob = BigInt::zero();

        for i in 0..kappa {
            let bin_coeff = binomial_num(cur_ell, i);
            let term1 = BigInt::from(2).pow(i);
            let term2 = BigInt::from(3).pow(cur_ell - i);
            badprob += bin_coeff * term1 * term2;
        }

        badprob /= BigInt::from(4).pow(cur_ell);

        println!("ell = {}, badprob = {}", cur_ell, badprob);

        if badprob <= negl {
            println!("found ell = {}", cur_ell);
            ell = Some(cur_ell);
        }
    }

    ell
}

fn binomial_num(n: u32, k: u32) -> num_bigint::BigInt {
    use num_bigint::BigInt;
    use num_traits::One;

    let mut res = BigInt::one();

    for i in 1..=k {
        res *= n - i + 1;
        res /= i;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_length_search() {
        assert_eq!(key_length_search_num(1700, 1800).unwrap(), 42);
    }
}
