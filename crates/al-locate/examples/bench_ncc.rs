use al_locate::dsp;
use al_locate::methods::{BandNcc, FullNcc, GccPhat, LocateMethod, SpectralEnvelope};
fn main() {
    let signal: Vec<f64> = (0..2_200_000)
        .map(|i| (i as f64 * 0.0001).sin() * 0.5)
        .collect();
    let template: Vec<f64> = signal[500_000..510_000].to_vec(); // ~0.2s SE-like
    let t = std::time::Instant::now();
    let _ = FullNcc.locate(&template, &signal, 44100);
    println!("FullNcc: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let _ = BandNcc.locate(&template, &signal, 44100);
    println!("BandNcc: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let _ = GccPhat.locate(&template, &signal, 44100);
    println!("GccPhat: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let _ = SpectralEnvelope::default().locate(&template, &signal, 44100);
    println!("SpecEnv: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let _ = dsp::bandpass(&signal, 44100, 250.0, 1000.0);
    println!("bandpass(sig): {:?}", t.elapsed());
}
