use google_keep1::hyper;
use google_keep1::hyper_rustls;
use google_keep1::Keep;

async fn foo() -> Vec<ListItem> {
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper::Client::builder().build(connector);
    let mut hub = Keep::new(client, auth);
    let (response, note) = hub.notes().get("foo").doit().await?;
    let body = note.body?;
    let list = body.list?;
    list.list_items?
}
