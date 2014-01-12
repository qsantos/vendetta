/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <locale.h>

#include "game.h"

static void usage(const char* name)
{
	fprintf(stderr,
		"Usage: %s [options]\n"
		"\n"
		"options:\n"
		"  --help        print this help\n"
		"  --version     print version information\n"
		"  --quickstart  give player 1000 of each material\n"
		"  --godmode     give player god-like skills\n"
		, name
	);
	exit(1);
}
int main(int argc, char** argv)
{
	setlocale(LC_ALL, "");
	setlocale(LC_NUMERIC, "C");

	char god_mode    = 0;
	char quick_start = 0;

	int curarg = 1;
	while (curarg < argc)
	{
		const char* option = argv[curarg++];
		if (strcmp(option, "--help") == 0)
		{
			usage(argv[0]);
		}
		else if (strcmp(option, "--version") == 0)
		{
			fprintf(stderr, "Vendetta version 0.2\n");
			fprintf(stderr, "Compiled on %s at %s\n", __DATE__, __TIME__);
			exit(1);
		}
		else if (strcmp(option, "--quickstart") == 0)
		{
			quick_start = 1;
		}
		else if (strcmp(option, "--godmode") == 0)
		{
			god_mode = 1;
		}
		else
		{
			fprintf(stderr, "Unknown option '%s'\n", option);
			usage(argv[0]);
		}
	}

	game_t game;
	game_init(&game);

	if (quick_start)
	{
		float* m = game.player->inventory.materials;
		for (int i = 0; i < game.u->n_materials; i++)
			m[i] = 1000;
	}
	if (god_mode)
	{
		float* s= game.player->sskills;
		for (int i = 0; i < N_SPECIAL_SKILLS; i++)
			s[i] = 1000;
		s = game.player->mskills;
		for (int i = 0; i < game.u->n_materials; i++)
			s[i] = 1000;
		s = game.player->iskills;
		for (int i = 0; i < game.u->n_iskills; i++)
			s[i] = 1000;
	}

	game_loop(&game);
	game_exit(&game);
	return 0;
}
