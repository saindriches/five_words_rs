## five_words_rs
A solution to the problem of finding five English words with 25 unique characters, written in Rust.

See video by Matt Parker: https://www.youtube.com/watch?v=_-AfhLQfb6w

---
### Build

```bash
cargo build --release
```
The project requires a nightly toolchain, it will be installed automatically if you don't have one.

---
### Run
Get [words_alpha.txt](https://github.com/dwyl/english-words/blob/master/words_alpha.txt) first.
```bash
./target/release/five_words_rs ./words_alpha.txt
```
You can also speficy more arguments, such as:
```bash
./target/release/five_words_rs ./words_alpha.txt 4 6
```
... to find 4 6-letter words with 24 unique characters.

---
### Performance

Some results on my machines:

#### Apple M1 Pro @ 3.20 GHz
```
Loop depth: 0, matches: 315, elapsed: 276.833µs
Loop depth: 1, matches: 20051, elapsed: 1.633708ms
Loop depth: 2, matches: 130179, elapsed: 13.274625ms
Loop depth: 3, matches: 55619, elapsed: 16.796375ms
Loop depth: 4, matches: 538, elapsed: 6.770041ms
Matches with anagrams: 293, total matches: 831
------------------------------------------------------------
File reading elapsed: 7.712166ms
Frequency analysing elapsed: 326.416µs
Matching elapsed: 38.475416ms
Printing elapsed: 1.465708ms
Total elapsed: 48.262958ms
```

---
#### Apple M1 @ 3.20 GHz
````
Loop depth: 0, matches: 315, elapsed: 506.041µs
Loop depth: 1, matches: 20051, elapsed: 1.52975ms
Loop depth: 2, matches: 130179, elapsed: 18.288583ms
Loop depth: 3, matches: 55619, elapsed: 27.188083ms
Loop depth: 4, matches: 538, elapsed: 12.427083ms
Matches with anagrams: 293, total matches: 831
------------------------------------------------------------
File reading elapsed: 17.158166ms
Frequency analysing elapsed: 630.291µs
Matching elapsed: 59.434416ms
Printing elapsed: 2.46625ms
Total elapsed: 80.244583ms
````
----

#### AMD Ryzen 7 1700X @ 3.40GHz
````
Loop depth: 0, matches: 315, elapsed: 3.7011ms
Loop depth: 1, matches: 20051, elapsed: 7.5651ms
Loop depth: 2, matches: 130179, elapsed: 28.2892ms
Loop depth: 3, matches: 55619, elapsed: 25.1168ms
Loop depth: 4, matches: 538, elapsed: 8.7907ms
Matches with anagrams: 293, total matches: 831
------------------------------------------------------------
File reading elapsed: 10.3677ms
Frequency analysing elapsed: 506.4µs
Matching elapsed: 69.7699ms
Printing elapsed: 27.339ms
Total elapsed: 111.8984ms
````
----
#### Intel Core i7-3770 @ 3.40 GHz
````
Loop depth: 0, matches: 315, elapsed: 695.416µs
Loop depth: 1, matches: 20051, elapsed: 8.12513ms
Loop depth: 2, matches: 130179, elapsed: 62.82143ms
Loop depth: 3, matches: 55619, elapsed: 94.809048ms
Loop depth: 4, matches: 538, elapsed: 38.090651ms
Matches with anagrams: 293, total matches: 831
------------------------------------------------------------
File reading elapsed: 23.199599ms
Frequency analysing elapsed: 1.074597ms
Matching elapsed: 203.860416ms
Printing elapsed: 31.331657ms
Total elapsed: 260.374916ms
````