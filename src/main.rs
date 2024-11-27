use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut spider = BanzhuSpider::new();
    spider.run().await?;

    Ok(())
}


