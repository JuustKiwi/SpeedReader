.PHONY: install clean

install:
	g++ -O2 -c extractors.cpp -o extractors.o -I/usr/include/poppler/cpp -I/usr/include/poppler
	g++ -O2 -c engine.cpp -o engine.o
	ar rcs libengine.a extractors.o engine.o
	cargo build --release
	sudo cp target/release/SpeedReader /usr/local/bin/speedreader

clean:
	rm -f *.o *.a
