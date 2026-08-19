.PHONY: install

install:
	g++ -O2 -c engine.cpp -o engine.o -I/usr/include/poppler/cpp -I/usr/include/poppler
	ar rcs libengine.a engine.o
	cargo build --release
	sudo cp target/release/SpeedReader /usr/local/bin/speedreader
