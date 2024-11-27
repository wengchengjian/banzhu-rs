> 提醒本项目仅用于rust爬虫学习,本身程序还有许多问题, 并发度, 仍有部分内容解析失败等等。
> 本项目涉及三种语言互操作, `Pyhton`,`Rust`,`JS`, 代码结构比较混乱

### 版主爬虫
> 由于Rust本身关于无头浏览器的库很少, 对于有关隐藏性的加强就更少了, 
> 所以使用rust本身的库很难过`cloudflare`的自动化工具检测, 于是采用的是`python`的`DrissionPage`来应对`cloudflare`的检测

#### 爬取流程

1. 使用`python`库`DrissionPage` 过自动化工具检测
2. 使用`opencv` 识别`cloudflare`人机检测，同时读取`cookie`,`user-agent`
3. 开始爬取`url`
4. 解析`html`, 解决图片反爬, 字体反爬, js混淆, aes解密
5. 保存文件
#### 依赖环境
##### nodejs

- jsdom
##### py

- DrissionPage
- execjs

##### rust

##### opencv


#### 图片和字体反爬
> 简单来讲图片利用图像识别来做, 然后就有了图片和文字的对应表

#### aes 解密
> 这个网站把密钥放前端了,前16位iv, 后16位key 所以很简单就能解密了

#### TODO

- 结合命令行可以写一个能搜索下载的工具