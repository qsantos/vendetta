CC      = gcc
CFLAGS  = -Wall -Wextra -Werror -pedantic -ansi -O3 -std=c99
LDFLAGS = -O3 -lcsfml-graphics -lcsfml-window -lcsfml-system
TARGETS = vendetta

all: $(TARGETS)

vendetta: main.o
	$(CC) $(LDFLAGS) $^ -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f *.o

destroy: clean
	rm -f $(TARGETS)

rebuild: destroy all
