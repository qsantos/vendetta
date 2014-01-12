CC      = gcc
CFLAGS  = -Wall -Wextra -Werror -pedantic -ansi -O3 -std=c99 -D_GNU_SOURCE
LDFLAGS = -O3 -lcsfml-graphics -lcsfml-window -lcsfml-system -lm
TARGETS = vendetta

SRC = $(shell find -name "*.c")
OBJ = $(SRC:.c=.o)

all: $(TARGETS)

vendetta: $(OBJ)
	@echo $(CC) [...] $(LDFLAGS) -o $@
	@$(CC) $^ $(LDFLAGS) -o $@

-include $(OBJ:.o=.d)

%.o: %.c
	$(CC) $(CFLAGS) -c $*.c -o $*.o
	@$(CC) -MM -MT $@ $(CFLAGS) $*.c > $*.d

clean:
	@echo rm -f [*.o] [*.d]
	@rm -f $(OBJ) $(OBJ:.o=.d)

destroy: clean
	rm -f $(TARGETS)

rebuild: destroy all
