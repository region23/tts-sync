mod models;
mod analysis;
pub mod adjustment;
pub mod utils;

pub use models::{AudioData, AudioSegment, AudioTrack};
pub use analysis::{
    AudioAnalyzer, AudioAnalysis, SegmentAnalysis, SilenceSegment
};
pub use adjustment::tempo::{TempoAdjuster, TempoAlgorithm};
pub use adjustment::synchronizer::AudioSynchronizer;
pub use adjustment::processor::AudioProcessor;
pub use utils::{decode_mp3_to_samples};
