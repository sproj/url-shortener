use crate::common::{constants::API_PATH_SHORTEN, test_app};
use hyper::StatusCode;
use url_shortener::domain::models::short_url::ShortUrl;

pub mod common;

#[tokio::test]
async fn create_short_from_input_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let url = sut.build_path(API_PATH_SHORTEN);

    let client = reqwest::Client::new();

    let expected = "http://create.me";

    let res = client.post(url).json(expected).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let actual = res.json::<ShortUrl>().await.unwrap();
    assert_eq!(actual.long_url, expected)
}

#[tokio::test]
async fn get_after_create_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let create_url = sut.build_path(API_PATH_SHORTEN);
    let client = reqwest::Client::new();

    let expected = "http://read.me";

    let create = client
        .post(create_url)
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let create_result = create.json::<ShortUrl>().await.unwrap();
    let created_id = create_result.id;
    let get_by_id_url = sut.build_path(format!("{}/{}", API_PATH_SHORTEN, created_id).as_str());

    let read = client.get(get_by_id_url).send().await.unwrap();

    assert_eq!(read.status(), StatusCode::OK);

    let actual = read.json::<ShortUrl>().await.unwrap();

    assert_eq!(actual.long_url, expected)
}

#[tokio::test]
async fn delete_by_id_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let create_url = sut.build_path(API_PATH_SHORTEN);
    let client = reqwest::Client::new();

    let expected = "http://delete.me";

    let create = client
        .post(create_url)
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);

    let actual = create.json::<ShortUrl>().await.unwrap();
    let url_with_id_path_param =
        sut.build_path(format!("{}/{}", API_PATH_SHORTEN, actual.id).as_str());

    let delete = client.delete(url_with_id_path_param).send().await.unwrap();

    assert_eq!(delete.status(), StatusCode::OK);

    let delete_response = delete.json::<bool>().await.unwrap();
    assert_eq!(delete_response, true);
}
