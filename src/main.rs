use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use sha3::{Digest, Sha3_512};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinError;
use warp::Filter;

const SLEEP_TIME: Duration = Duration::from_millis(200);
const RUN_ADDRESS: ([u8; 4], u16) = ([127, 0, 0, 1], 3030);

#[tokio::main]
async fn main() -> Result<(), JoinError> {
    let hasher = Arc::new(Mutex::new(Sha3_512::new()));
    tokio::join!(
        // Spawn the webserver which will broadcast the latest task
        {
            let hasher = hasher.clone();
            let web_server = warp::path!()
                .map(move || format!("{:?}", hasher.lock().unwrap().clone().finalize(),));
            warp::serve(web_server).run(RUN_ADDRESS)
        },
        // Spawn the webcam capture
        tokio::spawn(async move {
            loop {
                {
                    let index = CameraIndex::Index(0);
                    let requested = RequestedFormat::new::<RgbFormat>(
                        RequestedFormatType::AbsoluteHighestFrameRate,
                    );
                    let mut camera = Camera::new(index, requested).unwrap();
                    let frame = camera.frame().unwrap();
                    println!("{:?}", frame_info::FrameInfo::try_new(&frame).unwrap());
                    hasher.lock().unwrap().update(frame.buffer());
                }
                tokio::time::sleep(SLEEP_TIME).await;
            }
        })
    )
    .1
}

mod frame_info {
    use std::fmt::Debug;

    use image::Pixel;
    use nokhwa::{pixel_format::RgbFormat, Buffer, NokhwaError};

    #[derive(PartialEq, Copy, Clone)]
    pub struct FrameInfo {
        brightness: f32,
    }

    impl FrameInfo {
        pub fn try_new(frame: &Buffer) -> Result<FrameInfo, NokhwaError> {
            let decoded = frame.decode_image::<RgbFormat>()?;
            let total_brightness = decoded
                .pixels()
                .map(|p| p.to_luma().0[0])
                .map(u32::from)
                .sum::<u32>() as f32;
            Ok(FrameInfo {
                brightness: total_brightness / decoded.len() as f32 / 256f32,
            })
        }
    }

    impl Debug for FrameInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            const BAR_SIZE: usize = 80;
            let len = (BAR_SIZE as f32 * self.brightness) as usize;
            write!(f, "Brightness: {:.3}:\t", self.brightness)?;
            for _ in 0..len {
                write!(f, "█")?;
            }
            Ok(())
        }
    }
}
