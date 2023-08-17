use async_recursion::async_recursion;
use std::path::PathBuf;
use tokio::{
    fs::{copy, create_dir_all, read_dir, remove_dir_all, try_exists, File},
    io::{self, AsyncWriteExt},
};

#[async_recursion]
pub async fn copy_dir_all(from: PathBuf, to: PathBuf) -> io::Result<()> {
    create_dir_all(&to).await?;
    let mut dir = read_dir(from).await?;
    while let Some(child) = dir.next_entry().await? {
        if child.metadata().await?.is_dir() {
            copy_dir_all(child.path(), to.join(child.file_name())).await?;
        } else {
            copy(child.path(), to.join(child.file_name())).await?;
        }
    }
    Ok(())
}

pub async fn copy_dir_all_empty(from: PathBuf, to: PathBuf) -> io::Result<()> {
    if try_exists(to.clone()).await? {
        remove_dir_all(to.clone()).await?;
    }

    copy_dir_all(from, to).await?;
    Ok(())
}

pub async fn write_file_str(path: PathBuf, str: &str) -> io::Result<()> {
    File::create(path.clone())
        .await?
        .write_all(str.as_bytes())
        .await?;
    Ok(())
}
