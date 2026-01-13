use yaiko_core::{
    App, Router, Server, Request, Response,
    LoggingMiddleware, SecurityHeaders, init_tracing, tracing,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();
    
    let router = Router::new()
        .get("/", index_handler)
        .static_files("/css", "./public/css")
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());
    
    let app = App::new().router(router);
    
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    tracing::info!("Webinar server running at http://{}", addr);
    server.run().await?;
    
    Ok(())
}

async fn index_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Build Your AI Agency</title>
    <link rel="stylesheet" href="/css/style.css">
    <link href="https://fonts.googleapis.com/css2?family=Playfair+Display:wght@700&family=Lato:wght@400;700&display=swap" rel="stylesheet">
    <script async defer src="https://js.whop.com/static/checkout/loader.js"></script>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo">AI AGENCY MASTERCLASS</div>
        </header>

        <main>
            <h1 class="headline">How to Start a 6-Figure AI Agency in 30 Days</h1>
            <h2 class="subheadline">Without Coding Skills or Previous Experience</h2>

            <div class="video-wrapper">
                <div class="video-placeholder">
                    <div class="play-button">&#9658;</div>
                    <p>Watch the Free Training</p>
                </div>
            </div>

            <div class="content-section">
                <h3>In This Exclusive Training, You Will Learn:</h3>
                <ul class="benefits-list">
                    <li>The exact 3-step framework to land your first $5,000 client.</li>
                    <li>How to automate service delivery using AI tools.</li>
                    <li>Why now is the perfect time to start an AI agency.</li>
                    <li>Case studies of students who quit their 9-5 jobs.</li>
                </ul>
            </div>
            
            <div class="checkout-section">
                <h3>Ready to Join?</h3>
                <p>Get instant access to the full course and community.</p>
                
                <!-- Whop Checkout Embed -->
                <div class="whop-embed-container">
                    <div
                        data-whop-checkout-plan-id="plan_foOMXso5sInTG"
                        data-whop-checkout-theme="light"
                        data-whop-checkout-hide-price="false"
                        style="height: fit-content; overflow: hidden; width: 100%; max-width: 500px; margin: 0 auto;"
                    ></div>
                </div>
            </div>
        </main>

        <footer>
            <p>&copy; 2024 AI Agency Masterclass. All rights reserved.</p>
            <p><a href="#">Terms</a> | <a href="#">Privacy</a></p>
        </footer>
    </div>
</body>
</html>"##;

    Ok(Response::new().html(html))
}
