use rand::{Rng, RngExt};

pub(crate) fn laplace<R: Rng + ?Sized>(scale: f64, rng: &mut R) -> f64 {
    let u = rng.random::<f64>() - 0.5;
    -scale * u.signum() * (-2. * u.abs()).ln_1p()
}
// CDF inversion draws the same conditional Laplace law as bounded rejection
// sampling, without an unbounded rejection loop.
pub(crate) fn bounded<R: Rng + ?Sized>(
    value: f64,
    lo: f64,
    hi: f64,
    epsilon: f64,
    rng: &mut R,
) -> f64 {
    bounded_with_sensitivity(value, lo, hi, hi - lo, epsilon, rng)
}

// Explicit sensitivity for timestamp shifts. Never reduce it below the
// admissible domain diameter, including fallback intervals wider than the log.
pub(crate) fn bounded_with_sensitivity<R: Rng + ?Sized>(
    value: f64,
    lo: f64,
    hi: f64,
    sensitivity: f64,
    epsilon: f64,
    rng: &mut R,
) -> f64 {
    if lo == hi {
        return lo;
    }
    let value = value.clamp(lo, hi);
    // At or above the diameter, use the supplied conservative sensitivity.
    // This preserves the conditional Laplace law without reducing the noise
    // scale to the narrower admissible timestamp interval.
    let scale = sensitivity.max(hi - lo) / epsilon;
    let cdf = |x: f64| {
        if x < value {
            0.5 * ((x - value) / scale).exp()
        } else {
            1. - 0.5 * (-(x - value) / scale).exp()
        }
    };
    let lower = cdf(lo);
    let upper = cdf(hi);
    let u = lower + (upper - lower) * rng.random::<f64>();
    let result = if u <= 0.5 {
        value + scale * (2. * u).ln()
    } else {
        value - scale * (2. * (1. - u)).ln()
    };
    result.clamp(lo, hi)
}
pub(crate) fn categorical<R: Rng + ?Sized>(
    value: usize,
    size: usize,
    epsilon: f64,
    rng: &mut R,
) -> usize {
    let other = (-epsilon).exp();
    let mut draw = rng.random::<f64>() * (1. + (size - 1) as f64 * other);
    for i in 0..size {
        draw -= if i == value { 1. } else { other };
        if draw < 0. {
            return i;
        }
    }
    size - 1
}
// pm4py SaCoFa's exp_mech helper exponentiates exponential scores again. Preserve
// this double-exponential law (rather than silently replacing it with softmax).
pub(crate) fn universe<R: Rng + ?Sized>(size: usize, epsilon: f64, rng: &mut R) -> usize {
    let raw: Vec<_> = (0..=size)
        .map(|i| ((size - i) as f64 * epsilon / 2.).exp().min(f64::MAX))
        .collect();
    let weights: Vec<_> = raw.iter().map(|v| (v - raw[0]).exp()).collect();
    let mut draw = rng.random::<f64>() * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        draw -= w;
        if draw < 0. {
            return i;
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    #[test]
    fn mechanism_distribution_goldens() {
        for id in [
            "privacy-mechanisms",
            "privacy-mechanisms-0-5",
            "privacy-mechanisms-2-0",
        ] {
            let g = ichnos_golden::golden("simulation", id);
            let e = &g.expected;
            let epsilon = e["epsilon"].as_f64().unwrap();
            let n = e["samples"].as_u64().unwrap() as usize;
            let mut rng = ChaCha8Rng::seed_from_u64(1729);
            let mut numeric = 0.;
            let mut binary = 0;
            let mut category = 0;
            let mut zero = 0;
            let mut universe_zero = 0;
            for _ in 0..n {
                let v = bounded(2., 0., 10., epsilon, &mut rng);
                assert!((0. ..=10.).contains(&v));
                numeric += v;
                binary += usize::from(rng.random::<f64>() < 1. / (1. + (-epsilon).exp()));
                category += usize::from(categorical(0, 3, epsilon, &mut rng) == 0);
                zero += usize::from(laplace(1. / epsilon, &mut rng).trunc() == 0.);
                universe_zero += usize::from(universe(3, epsilon, &mut rng) == 0);
            }
            for (key, actual, tol) in [
                ("numeric_mean", numeric / n as f64, 0.12),
                ("boolean_keep", binary as f64 / n as f64, 0.025),
                ("categorical_keep", category as f64 / n as f64, 0.025),
                ("integer_laplace_zero", zero as f64 / n as f64, 0.025),
                ("universe_zero", universe_zero as f64 / n as f64, 0.025),
            ] {
                let expected = e[key].as_f64().unwrap();
                assert!(
                    (actual - expected).abs() <= tol,
                    "{key}: {actual} vs {expected}"
                );
            }
        }
    }
}
