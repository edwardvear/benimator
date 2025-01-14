use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    time::Duration,
};

use serde::{
    de::{self, value::MapAccessDeserializer, MapAccess, Unexpected},
    Deserialize,
};

use super::{Frame, Mode, SpriteSheetAnimation};

#[derive(Deserialize)]
pub(super) struct AnimationDto {
    #[serde(default)]
    mode: ModeDto,
    #[serde(default)]
    frame_duration: Option<u64>,
    frames: Vec<FrameDto>,
}

#[derive(Deserialize)]
enum ModeDto {
    Repeat,
    RepeatFrom(usize),
    Once,
    PingPong,
}

impl Default for ModeDto {
    fn default() -> Self {
        ModeDto::Repeat
    }
}

struct FrameDto {
    index: usize,
    duration: Option<u64>,
}

impl<'de> Deserialize<'de> for FrameDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct FrameDtoMap {
            index: usize,
            duration: Option<u64>,
        }

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = FrameDto;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "either a frame index, or a frame-index with a")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.try_into()
                    .map(|index| FrameDto {
                        index,
                        duration: None,
                    })
                    .map_err(|_| de::Error::invalid_value(Unexpected::Unsigned(v), &self))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let FrameDtoMap { index, duration } =
                    FrameDtoMap::deserialize(MapAccessDeserializer::new(map))?;
                Ok(FrameDto { index, duration })
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl TryFrom<AnimationDto> for SpriteSheetAnimation {
    type Error = InvalidAnimation;

    fn try_from(animation: AnimationDto) -> Result<Self, Self::Error> {
        Ok(Self {
            frames: animation
                .frames
                .into_iter()
                .map(|FrameDto { index, duration }| {
                    match duration.or(animation.frame_duration).filter(|d| *d > 0) {
                        Some(duration) => Ok(Frame::new(index, Duration::from_millis(duration))),
                        None => Err(InvalidAnimation::ZeroDuration),
                    }
                })
                .collect::<Result<_, _>>()?,
            mode: match animation.mode {
                ModeDto::Repeat => Mode::RepeatFrom(0),
                ModeDto::RepeatFrom(f) => Mode::RepeatFrom(f),
                ModeDto::Once => Mode::Once,
                ModeDto::PingPong => Mode::PingPong,
            },
        })
    }
}

#[derive(Debug)]
pub(super) enum InvalidAnimation {
    ZeroDuration,
}

impl Display for InvalidAnimation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InvalidAnimation::ZeroDuration => write!(f, "invalid duration, must be > 0"), /*  */
        }
    }
}

impl Error for InvalidAnimation {}

impl SpriteSheetAnimation {
    /// Parse content of a yaml string representing the animation
    ///
    /// # Yaml schema
    ///
    /// ```yaml
    /// # The mode can be one of: 'once', 'repeat', 'ping-pong'
    /// # or 'repeat-from: n' (where 'n' is the frame-index to repeat from)
    /// # The default is 'Repeat'
    /// mode: PingPong
    /// frames:
    ///   - index: 0 # index in the sprite sheet for that frame
    ///     duration: 100 # duration of the frame in milliseconds
    ///   - index: 1
    ///     duration: 100
    ///   - index: 2
    ///     duration: 120
    /// ```
    ///
    /// There is also a short-hand notation if all frames have the same duration:
    /// ```yaml
    /// frame_duration: 100
    /// frames: [0, 1, 2] # sequence of frame indices
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the content is not a valid yaml representation of an animation
    #[cfg(feature = "yaml")]
    pub fn from_yaml_str(yaml: &str) -> Result<Self, AnimationParseError> {
        Self::from_yaml_bytes(yaml.as_bytes())
    }

    /// Parse content of yaml bytes representing the animation
    ///
    /// # Yaml schema
    ///
    /// ```yaml
    /// # The mode can be one of: 'Once', 'Repeat', 'PingPong'
    /// # or 'RepeatFrom: n' (where 'n' is the frame-index to repeat from)
    /// # The default is 'Repeat'
    /// mode: PingPong
    /// frames:
    ///   - index: 0 # index in the sprite sheet for that frame
    ///     duration: 100 # duration of the frame in milliseconds
    ///   - index: 1
    ///     duration: 100
    ///   - index: 2
    ///     duration: 120
    /// ```
    ///
    /// There is also a short-hand notation if all frames have the same duration:
    /// ```yaml
    /// frame-duration: 100
    /// frames: [0, 1, 2] # sequence of frame indices
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the content is not a valid yaml representation of an animation
    #[cfg(feature = "yaml")]
    pub fn from_yaml_bytes(yaml: &[u8]) -> Result<Self, AnimationParseError> {
        yaml::from_slice(yaml).map_err(AnimationParseError::new)
    }

    /// Parse content of a ron file represenging the animation
    ///
    /// # Schema
    ///
    /// ```ron
    /// (
    ///   // The mode can be one of: 'Once', 'Repeat', 'PingPong'
    ///   // or 'RepeatFrom(n)' (where 'n' is the frame-index to repeat from)
    ///   // The default is 'Repeat'
    ///   mode: PingPong,
    ///   frames: [
    ///     (
    ///       index: 0, //index in the sprite sheet for that frame
    ///       duration: Some(100), // duration of the frame in milliseconds
    ///     ),
    ///     (index: 1, duration: Some(100)),
    ///     (index: 2, duration: Some(120)),
    ///   ]
    /// )
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the content is not a valid ron representation of an animation
    #[cfg(feature = "ron")]
    pub fn from_ron_str(ron: &str) -> Result<Self, AnimationParseError> {
        Self::from_ron_bytes(ron.as_bytes())
    }

    /// Parse content of a ron file represenging the animation
    ///
    /// # Schema
    ///
    /// ```ron
    /// (
    ///   // The mode can be one of: 'Once', 'Repeat', 'PingPong'
    ///   // or 'RepeatFrom(n)' (where 'n' is the frame-index to repeat from)
    ///   // The default is 'Repeat'
    ///   mode: PingPong,
    ///   frames: [
    ///     (
    ///       index: 0, //index in the sprite sheet for that frame
    ///       duration: 100, // duration of the frame in milliseconds
    ///     ),
    ///     (index: 1, duration: 100),
    ///     (index: 2, duration: 120),
    ///   ]
    /// )
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the content is not a valid ron representation of an animation
    #[cfg(feature = "ron")]
    pub fn from_ron_bytes(ron: &[u8]) -> Result<Self, AnimationParseError> {
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_bytes(ron)
            .map_err(AnimationParseError::new)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct AnimationParseError(anyhow::Error);

impl AnimationParseError {
    fn new(err: impl Error + Send + Sync + 'static) -> Self {
        Self(anyhow::Error::from(err))
    }
}

impl Display for AnimationParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Animation format is invalid: {}", self.0)
    }
}

impl Error for AnimationParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "yaml")]
    mod yaml {
        use super::*;

        #[test]
        fn parse() {
            // given
            let content = "
            mode: PingPong
            frames:
              - index: 0 # index in the sprite sheet for that frame
                duration: 100 # duration of the frame in milliseconds
              - index: 1
                duration: 100
              - index: 2
                duration: 120";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::PingPong);
            assert_eq!(
                animation.frames,
                vec![
                    Frame::new(0, Duration::from_millis(100)),
                    Frame::new(1, Duration::from_millis(100)),
                    Frame::new(2, Duration::from_millis(120)),
                ]
            );
        }

        #[test]
        fn default_mode() {
            // given
            let content = "
            frames:
              - index: 0
                duration: 100";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::RepeatFrom(0));
        }

        #[test]
        fn repeat() {
            // given
            let content = "
            mode: Repeat
            frames:
              - index: 0
                duration: 100";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::RepeatFrom(0));
        }

        #[test]
        fn once() {
            // given
            let content = "
            mode: Once
            frames:
              - index: 0
                duration: 100";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::Once);
        }

        #[test]
        fn repeat_from() {
            // given
            let content = "
            mode:
              RepeatFrom: 1
            frames:
              - index: 0
                duration: 100
              - index: 1
                duration: 100";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::RepeatFrom(1));
        }

        #[test]
        fn zero_duration() {
            // given
            let content = "
            frames:
              - index: 0
                duration: 0";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content);

            // then
            assert!(animation.is_err());
        }

        #[test]
        fn same_duration_for_all_frames() {
            // given
            let content = "
            frame_duration: 100
            frames:
              - index: 0
              - index: 1
              - index: 2
                duration: 200
        ";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(
                animation.frames,
                vec![
                    Frame::new(0, Duration::from_millis(100)),
                    Frame::new(1, Duration::from_millis(100)),
                    Frame::new(2, Duration::from_millis(200)),
                ]
            );
        }

        #[test]
        fn same_duration_for_all_frames_short_hand() {
            // given
            let content = "
            frame_duration: 100
            frames: [0, 1, 2]
        ";

            // when
            let animation = SpriteSheetAnimation::from_yaml_str(content).unwrap();

            // then
            assert_eq!(
                animation.frames,
                vec![
                    Frame::new(0, Duration::from_millis(100)),
                    Frame::new(1, Duration::from_millis(100)),
                    Frame::new(2, Duration::from_millis(100)),
                ]
            );
        }
    }

    #[cfg(feature = "ron")]
    mod ron {
        use super::*;

        #[test]
        fn frames() {
            // given
            let content = "
            (
                mode: RepeatFrom(1),
                frames: [
                    (
                        index: 0, // index in the sprite sheet for that frame
                        duration: 100, // # duration of the frame in milliseconds
                    ),
                    (index: 1, duration: 100),
                    (index: 2, duration: 120),
                ]
            )";

            // when
            let animation = SpriteSheetAnimation::from_ron_str(content).unwrap();

            // then
            assert_eq!(animation.mode, Mode::RepeatFrom(1));
            assert_eq!(
                animation.frames,
                vec![
                    Frame::new(0, Duration::from_millis(100)),
                    Frame::new(1, Duration::from_millis(100)),
                    Frame::new(2, Duration::from_millis(120)),
                ]
            );
        }
    }
}
