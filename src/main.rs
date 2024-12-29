use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    
    let mut spider = BanzhuSpider::new();
    spider.run().await?;

    Ok(())
}


