#include "game.h"

int main(void)
{
	game_t game;
	game_init(&game);
	game_loop(&game);
	game_exit(&game);
	return 0;
}
