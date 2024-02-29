use std::{
    fs::{self, File},
    io::Write,
    ops::Add,
    path::Path,
    time::Duration,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_path: String,
    #[arg(short, long, default_value = "./subtitles.srt")]
    output_path: String,
}

#[derive(Debug, PartialEq)]
struct SrtBlock {
    index: usize,
    start_time_string: String,
    end_time_string: String,
    text: String,
}

fn main() {
    // コマンドライン引数から音声とテキストが入ったパスを受け取る
    let args = Args::parse();
    let input_path = Path::new(&args.input_path);
    let output_path = Path::new(&args.output_path);

    // wavとtxtを取り出す
    let files = extract_wav_and_txt(input_path);

    // srtのブロック情報を作成する
    let srt_blocks = make_srt_blocks(files);

    // srtファイル作成
    make_srt(srt_blocks, output_path);
}

fn extract_wav_and_txt(path: &Path) -> Vec<std::path::PathBuf> {
    // パスが存在しなければ異常終了
    // パスの中にwavまたはtxtが入っていなければ異常終了
    let files: Vec<std::path::PathBuf> = fs::read_dir(path)
        .expect("パスが存在しません")
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.is_file()
                && match path.extension() {
                    Some(ext) => ext == "wav" || ext == "txt",
                    None => false,
                }
        })
        .map(|entry| entry.path())
        .collect();

    let extensions: Vec<&str> = files
        .iter()
        .map(|p| p.extension().unwrap().to_str().unwrap())
        .collect();

    // パスの中にwavが入っていなければ異常終了
    let n_wav = extensions
        .iter()
        .filter(|ext| **ext == "wav")
        .collect::<Vec<&&str>>()
        .len();
    if n_wav == 0 {
        panic!("wavが存在しません");
    };

    // パスの中にtxtが入っていなければ異常終了
    let n_txt = extensions
        .iter()
        .filter(|ext| **ext == "txt")
        .collect::<Vec<&&str>>()
        .len();
    if n_txt == 0 {
        panic!("txtが存在しません");
    };

    // wavとtxtが同数でなければ異常終了
    if n_wav != n_txt {
        panic!("wavとtxtの数が合いません");
    }

    files
}

fn make_srt_blocks(files: Vec<std::path::PathBuf>) -> Vec<SrtBlock> {
    let mut blocks: Vec<SrtBlock> = Vec::new();
    let mut total_time = Duration::from_secs_f64(0.);

    // 連番を回しつつwavとtxtから情報を抜き出す
    for i in 0.. {
        // ファイル検索用連番取得
        let seq_char = format!("{:03}", i);

        // 対象ブロックのファイル抽出
        let target_files: Vec<&std::path::PathBuf> = files
            .iter()
            .filter(|f| {
                f.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(&seq_char)
            })
            .collect();

        // ファイルを取得できなくなった時点で終了
        if target_files.len() == 0 {
            break;
        }

        // wavから開始と終了時間取得
        let wav_path = target_files
            .iter()
            .find(|p| p.extension().unwrap() == "wav")
            .unwrap();
        let mut inp_file = File::open(Path::new(wav_path)).unwrap();
        let (header, data) = wav::read(&mut inp_file).unwrap();

        let start_time_string = format!(
            "{:02}:{:02}:{:02},{:03}",
            total_time.as_secs() / 3600,
            (total_time.as_secs() % 3600) / 60,
            total_time.as_secs() % 60,
            total_time.subsec_millis()
        );

        let wav_duration = Duration::from_secs_f64(
            data.try_into_sixteen().unwrap().len() as f64 / header.sampling_rate as f64,
        );
        let end_time_duration = total_time.add(wav_duration);
        let end_time_string = format!(
            "{:02}:{:02}:{:02},{:03}",
            end_time_duration.as_secs() / 3600,
            (end_time_duration.as_secs() % 3600) / 60,
            end_time_duration.as_secs() % 60,
            end_time_duration.subsec_millis()
        );

        total_time = total_time.add(wav_duration);

        // txtからテキスト取得
        let txt_path = target_files
            .iter()
            .find(|p| p.extension().unwrap() == "txt")
            .unwrap();
        let text = fs::read_to_string(txt_path).unwrap();

        blocks.push(SrtBlock {
            index: i + 1,
            start_time_string,
            end_time_string,
            text,
        });
    }

    blocks
}

fn make_srt(srt_blocks: Vec<SrtBlock>, path: &Path) {
    let mut output_srt = String::new();

    // 書き出し用文字列作成
    for block in srt_blocks {
        output_srt.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            block.index, block.start_time_string, block.end_time_string, block.text
        ));
    }

    // 書き出し
    let mut file = File::create(path).unwrap();
    let _ = file.write_all(output_srt.trim_end().as_bytes());
}

#[test]
fn test_extract_wav_and_txt_ok() {
    let path = Path::new("./voice");
    extract_wav_and_txt(path);
}

#[test]
#[should_panic(expected = "パスが存在しません")]
fn test_extract_wav_and_txt_no_exits_path() {
    let path = Path::new("no/exits/path/");
    let _ = extract_wav_and_txt(path);
}

#[test]
#[should_panic(expected = "wavが存在しません")]
fn test_extract_wav_and_txt_no_wav() {
    let path = Path::new("test_resource/no_wav");
    extract_wav_and_txt(path);
}

#[test]
#[should_panic(expected = "txtが存在しません")]
fn test_extract_wav_and_txt_no_txt() {
    let path = Path::new("test_resource/no_txt");
    extract_wav_and_txt(path);
}

#[test]
#[should_panic(expected = "wavとtxtの数が合いません")]
fn test_extract_wav_and_txt_no_match() {
    let path = Path::new("test_resource/not_match");
    extract_wav_and_txt(path);
}

#[test]
fn test_make_srt_blocks_ok() {
    let path = Path::new("./voice");
    let files = extract_wav_and_txt(path);
    let srt_blocks = make_srt_blocks(files);

    let correct = vec!(
        SrtBlock { index: 1, start_time_string: "00:00:00,000".to_string(), end_time_string: "00:00:07,288".to_string(), text: "時は第三次中東戦争と第四次中東戦争の間の1973年2月初旬".to_string() },
        SrtBlock { index: 2, start_time_string: "00:00:07,288".to_string(), end_time_string: "00:00:13,722".to_string(), text: "エジプトを盟主とする中東アラブ諸国とイスラエルは、とてもピリピリした状態にありました".to_string() },
        SrtBlock { index: 3, start_time_string: "00:00:13,722".to_string(), end_time_string: "00:00:22,488".to_string(), text: "砂塵舞うベンガジ空港を飛び立ち、リビアン・アラブ航空114便は地中海を渡ってエジプトの首都カイロへ向かいます".to_string() },
        SrtBlock { index: 4, start_time_string: "00:00:22,488".to_string(), end_time_string: "00:00:31,547".to_string(), text: "コックピットにはフランス人機長、その右隣にフランス人航空機関士、後ろにはリビア人副操縦士が乗っていました".to_string() },
    );

    assert_eq!(correct[0], srt_blocks[0]);
    assert_eq!(correct[1], srt_blocks[1]);
    assert_eq!(correct[2], srt_blocks[2]);
    assert_eq!(correct[3], srt_blocks[3]);
}
