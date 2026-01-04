use std::time::Duration;

use rodio::Source as _;
use zng_txt::{Txt, formatx};
use zng_view_api::audio::{AudioMix, AudioMixLayer};

use crate::audio_cache::AudioCache;

impl AudioCache {
    pub(crate) fn vp_mix_to_source(
        &self,
        mix: AudioMix,
        channels: u16,
        sample_rate: u32,
    ) -> Result<Box<dyn rodio::Source + Send>, Txt> {
        let mut out = None::<Box<dyn rodio::Source + Send>>;
        fn box_source(source: impl rodio::Source + Send + 'static, skip: Duration, take: Duration) -> Box<dyn rodio::Source + Send> {
            if skip > Duration::ZERO {
                let source = source.skip_duration(skip);
                if take < Duration::MAX {
                    Box::new(source.take_duration(take))
                } else {
                    Box::new(source)
                }
            } else if take < Duration::MAX {
                Box::new(source.take_duration(take))
            } else {
                Box::new(source)
            }
        }
        fn mix_layer(prev: &mut Option<Box<dyn rodio::Source + Send>>, next: impl rodio::Source + Send + 'static) {
            if let Some(p) = prev.take() {
                *prev = Some(Box::new(p.mix(next)))
            } else {
                *prev = Some(Box::new(next));
            }
        }
        for layer in mix.layers {
            match layer {
                AudioMixLayer::Audio { audio, skip, take } => {
                    if let Some(s) = self.tracks.get(&audio) {
                        let s = s.play_source();
                        if s.channel_count != channels || s.sample_rate != sample_rate {
                            let s = rodio::source::UniformSourceIterator::new(s, channels, sample_rate);
                           if skip != Duration::ZERO || take != Duration::MAX {
                            let s = box_source(s, skip, take);
                            mix_layer(&mut out, s);
                        } else {
                            mix_layer(&mut out, s);
                        }
                        } else if skip != Duration::ZERO || take != Duration::MAX {
                            let s = box_source(s, skip, take);
                            mix_layer(&mut out, s);
                        } else {
                            mix_layer(&mut out, s);
                        }
                    } else {
                        return Err(formatx!("audio track {audio:?} not found"));
                    }
                }
                AudioMixLayer::AudioMix { mix, skip, take } => {
                    let mut layer = self.vp_mix_to_source(mix, channels, sample_rate)?;
                    if skip != Duration::ZERO || take != Duration::MAX {
                        layer = box_source(layer, skip, take);
                    }
                    mix_layer(&mut out, layer);
                }
                AudioMixLayer::VolumeLinear { start, end } => {
                    if start.0 < end.0 &&  let Some(s) = out.take() {
                        out = Some(Box::new(s.volume_linear(start.0, end.0, start.1.0, end.1.0)));
                    }
                }
                AudioMixLayer::SineWave { frequency, duration } => {
                    if duration > Duration::ZERO {
                        let s = rodio::source::SineWave::new(frequency).take_duration(duration);
                        mix_layer(&mut out, s);
                    }
                }
                _ => unreachable!(),
            }
        }

        match out {
            Some(o) => {
                if mix.delay > Duration::ZERO {
                    let o = o.delay(mix.delay);
                    if let Some(duration) = mix.total_duration {
                        let o = o.take_duration(duration);
                        Ok(Box::new(o))
                    } else {
                        Ok(Box::new(o))
                    }
                } else {
                    Ok(o)
                }
            },
            None => {
                // no layers just silent for the requested range
                let duration = match mix.total_duration {
                    Some(d) => mix.delay + d,
                    None => mix.delay,
                };
                let num_samples = (duration.as_secs_f64() * sample_rate as f64) as u64 * channels as u64;
                let silence = rodio::source::Zero::new_samples(channels, sample_rate, num_samples as usize);
                Ok(Box::new(silence))
            },
        }
    }
}

trait SourceExt: rodio::Source + Sized {
    /// Applies `map` while the stream is in `start..end` range.
    ///
    /// The map arguments are ([start, current, end], sample).
    fn range_map<F>(self, start: Duration, end: Duration, map: F) -> RangeMap<Self, F>
    where
        F: FnMut([u64; 3], f32) -> f32,
    {
        RangeMap::new(self, start, end, map)
    }

    /// Applies `map` while the stream is in `start..end` range.
    ///
    /// The map arguments are ([start, current, end], sample).
    fn range_map_norm<F>(self, start: Duration, end: Duration, mut map: F) -> impl rodio::Source
    where
        F: FnMut(f32, f32) -> f32,
    {
        self.range_map(start, end, move |[s, i, e], sample| map((i - s) as f32 / (e - s) as f32, sample))
    }

    fn volume_linear(self, start: Duration, end: Duration, start_volume: f32, end_volume: f32) -> impl rodio::Source {
        self.range_map_norm(start, end, move |t, s| s * lerp(start_volume, end_volume, t))
    }
}
impl<S: rodio::Source + Sized> SourceExt for S {}

struct RangeMap<S, F> {
    source: S,
    map: F,

    current_sample: u64,
    start_sample: u64,
    end_sample: u64,
}
impl<S, F> RangeMap<S, F>
where
    S: rodio::Source,
    F: FnMut([u64; 3], f32) -> f32,
{
    fn new(source: S, start: Duration, end: Duration, map: F) -> Self {
        let sample_rate = source.sample_rate() as u64;
        let channels = source.channels() as u64;

        let start_sample = (start.as_secs_f64() * sample_rate as f64) as u64 * channels;
        let end_sample = (end.as_secs_f64() * sample_rate as f64) as u64 * channels;

        Self {
            source,
            map,
            current_sample: 0,
            start_sample,
            end_sample,
        }
    }
}
impl<S, F> Iterator for RangeMap<S, F>
where
    S: rodio::Source,
    F: FnMut([u64; 3], f32) -> f32,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.source.next()?;
        let result = if self.current_sample >= self.start_sample && self.current_sample < self.end_sample {
            (self.map)([self.start_sample, self.current_sample, self.end_sample], sample)
        } else {
            sample
        };
        self.current_sample += 1;
        Some(result)
    }
}
impl<S, F> rodio::Source for RangeMap<S, F>
where
    S: rodio::Source,
    F: FnMut([u64; 3], f32) -> f32,
{
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.source.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

fn lerp(from: f32, to: f32, step: f32) -> f32 {
    from + (to - from) * step
}