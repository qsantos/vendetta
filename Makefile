CC      = gcc
CFLAGS  = -Wall -Wextra -Werror -pedantic -ansi -O3 -std=c99
LDFLAGS = -O3 -lcsfml-graphics -lcsfml-window -lcsfml-system -lm
TARGETS = vendetta

SRC = $(shell find -name "*.c")
OBJ = $(SRC:.c=.o)
HDR = $(shell find -name "*.h")
GCH = $(HDR:.h=.h.gch)

all: $(TARGETS)

headers: $(GCH)

%.h.gch: %.h
	$(CC) $(CFLAGS) -c $< -o $@

vendetta: $(OBJ)
	@echo $(CC) [...] $(LDFLAGS) -o $@
	@$(CC) $^ $(LDFLAGS) -o $@
-include $(OBJ:.o=.d)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@
	@$(CC) -MM -MT $@ $(CFLAGS) $< > $*.d

clean:
	@echo rm -f [*.o] [*.d]
	@rm -f $(OBJ) $(OBJ:.o=.d)

destroy: clean
	rm -f $(TARGETS)

rebuild: destroy all

cleanheaders:
	@echo rm -f [*.gch]
	@rm -f $(GCH)
