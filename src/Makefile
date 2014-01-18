CC      = gcc
CFLAGS  = -Wall -Wextra -Werror -pedantic -ansi -O3 -std=c99
LDFLAGS = -O3 -lcsfml-graphics -lcsfml-window -lcsfml-system -lm
TARGET  = ../vendetta

SRC = $(wildcard *.c */*.c)
OBJ = $(SRC:.c=.o)
HDR = $(wildcard *.h */*.h)
GCH = $(HDR:.h=.h.gch)

all: $(TARGET)

headers: $(GCH)

%.h.gch: %.h
	$(CC) $(CFLAGS) -c $< -o $@

$(TARGET): $(OBJ)
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

cleanall: destroy

rebuild: destroy
	@$(MAKE)

cleanheaders:
	@echo rm -f [*.gch]
	@rm -f $(GCH)
