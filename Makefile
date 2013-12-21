CC      = gcc
CFLAGS  = -Wall -Wextra -Werror -pedantic -ansi -O3 -std=c99
LDFLAGS = -O3 -lcsfml-graphics -lcsfml-window -lcsfml-system -lm
TARGETS = vendetta

SRC = $(wildcard *.c world/*.c universe/*.c)
OBJ = $(SRC:.c=.o)

all: $(TARGETS)

vendetta: $(OBJ)
	$(CC) $(LDFLAGS) $^ -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ)

destroy: clean
	rm -f $(TARGETS)

rebuild: destroy all
