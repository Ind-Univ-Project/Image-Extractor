use async_std::fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, File};
use async_std::io::ReadExt;
use async_std::io::{BufReader, WriteExt};
use async_std::path::{Path, PathBuf};
use async_std::stream::StreamExt;
use async_std::sync::{Arc};
use async_std::task;

use image::bmp::BmpEncoder as Encoder;
use image::ColorType;

use uuid::Uuid;

use std::time::Instant;

const IMAGE_SIZE: usize = 256 * 256 * 3;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let label_path = Path::new("./food_label.txt");
    let labels = read_to_string(label_path).await?;
    let labels: Vec<String> = labels
        .split('\n')
        .map(|e| e.replace("\r", "").to_string())
        .collect();

    let labels = Arc::new(labels);

    let src_dir = Path::new("./source");
    let mut files = read_dir(src_dir).await?;

    reset_dest_dir(labels.clone()).await;

    let start = Instant::now();
    
    let mut handles = Vec::new();
    loop {
        if let Some(file) = files.next().await {
            let file = file?;
            let handle = task::spawn(extract_image(file.path(), labels.clone()));

            handles.push(handle);
        }
        else {
            break;
        }
    }

    for handle in handles {
        handle.await.unwrap();
    }

    println!("{:?}", Instant::now().duration_since(start));
    Ok(())
}

async fn reset_dest_dir(labels: Arc<Vec<String>>) -> anyhow::Result<()> {
    remove_dir_all("./destination").await;
    let mut handles = Vec::new();

    for label in labels.iter() {
        handles.push(create_dir_all(format!("./destination/{}", label)));
    }

    for handle in handles {
        handle.await;
    }
    Ok(())
}

async fn extract_image(path: PathBuf, labels: Arc<Vec<String>>) -> anyhow::Result<()> {
    let current = path.to_str().unwrap().to_string();
    println!("Processing {}", current);

    let mut file = BufReader::new(File::open(path).await?);

    let mut handles = Vec::new();
    loop {
        
        let mut label = vec![0u8; 1];
        let mut img = vec![0u8; IMAGE_SIZE];

        if file.read_exact(&mut label).await.is_ok() {
            file.read_exact(&mut img).await?;
            let labels = labels.clone();


            let handle = task::spawn(async move {
                let label = &labels[label[0] as usize];
                let name = Uuid::new_v4();

                let mut result = Vec::new();

                let mut encoder = Encoder::new(&mut result);
                encoder.encode(&img, 256, 256, ColorType::Rgb8).unwrap();


                let mut file = File::create(format!("./destination/{}/{}.bmp", label, name))
                    .await
                    .expect(&format!("Creating {} failed", name));

                file.write_all(&result[0..result.len()]).await.unwrap();
                file.flush().await;
            });
            handles.push(handle)
        } else {
            break;
        }
    }

    for handle in handles {
        handle.await;
    }

    println!("Done {}", current);

    Ok(())
}
