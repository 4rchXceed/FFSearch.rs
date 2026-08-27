use crate::file_indexer::text_extractor::TextExtractor;

mod file_indexer;
mod searcher;
mod utils;

fn main() {
    let extractor = TextExtractor::new(1024, true, "eng");
    let result = extractor.extract(String::from(
        "C:\\Users\\Student\\Downloads\\media-504684426.jpg",
    ));
    let text = result.unwrap();
    println!("{}", text);
}
