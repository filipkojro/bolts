all: rust java

rust:
	cargo build --release

java: java/app/Main.java
	javac java/app/Main.java

run: all
	java -Djava.library.path=target/release -cp java app.Main
