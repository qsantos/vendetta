#include <locale.h>

#include "game.h"

int main(void)
{
	setlocale(LC_ALL, "");
	setlocale(LC_NUMERIC, "C");

	game_t game;
	game_init(&game);
	game_loop(&game);
	game_exit(&game);
	return 0;
}
