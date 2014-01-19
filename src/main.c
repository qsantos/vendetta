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
#include <time.h>

#include "game.h"

#define MAP_MIN_WIDTH  20
#define MAP_MIN_HEIGHT 20

static void usage(const char* name)
{
	fprintf(stderr,
		"Usage: %s [options]\n"
		"\n"
		"options:\n"
		"  -h, --help        print this help\n"
		"  -V, --version     print version information\n"
		"  -s, --size W H    set map's size to WxH tiles\n"
		"  -r, --seed seed   set generation seed\n"
		"                    if this parameter is omitted, the seed\n"
		"                    is generated from the current time\n"
		"  -q, --quickstart  give player 20 of each material\n"
		"  -g, --godmode     give player god-like skills\n"
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
	int  width  = 100;
	int  height = 100;
	unsigned int seed = time(NULL);

	int curarg = 1;
	while (curarg < argc)
	{
		const char* option = argv[curarg++];
		if (strcmp(option, "--help") == 0 || strcmp(option, "-h") == 0)
		{
			usage(argv[0]);
		}
		else if (strcmp(option, "--version") == 0 || strcmp(option, "-V") == 0)
		{
			fprintf(stderr, "Vendetta version 0.2\n");
			fprintf(stderr, "Compiled on %s at %s\n", __DATE__, __TIME__);
			exit(1);
		}
		else if (strcmp(option, "--size") == 0 || strcmp(option, "-s") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "No width given\n");
				usage(argv[0]);
			}
			width = strtol(argv[curarg++], NULL, 0);
			if (width < MAP_MIN_WIDTH)
			{
				fprintf(stderr, "Width must be at least %i (%i given)\n", MAP_MIN_WIDTH, width);
				usage(argv[0]);
			}
			if (curarg == argc)
			{
				fprintf(stderr, "No width given\n");
				usage(argv[0]);
			}
			height = strtol(argv[curarg++], NULL, 0);
			if (height < MAP_MIN_HEIGHT)
			{
				fprintf(stderr, "Height must be at least %i (%i given)\n", MAP_MIN_HEIGHT, height);
				usage(argv[0]);
			}
		}
		else if (strcmp(option, "--seed") == 0 || strcmp(option, "-r") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "No seed was given\n");
				usage(argv[0]);
			}
			seed = strtol(argv[curarg++], NULL, 0);
		}
		else if (strcmp(option, "--quickstart") == 0 || strcmp(option, "-q") == 0)
		{
			quick_start = 1;
		}
		else if (strcmp(option, "--godmode") == 0 || strcmp(option, "-g") == 0)
		{
			god_mode = 1;
		}
		else
		{
			fprintf(stderr, "Unknown option '%s'\n", option);
			usage(argv[0]);
		}
	}

	fprintf(stderr, "Using seed %#x\n", seed);

	game_t game;
	game_init(&game, width, height, seed);

	if (quick_start)
	{
		float* m = game.player->inventory.materials;
		for (size_t i = 0; i < game.u->n_materials; i++)
			m[i] = 20;
	}
	if (god_mode)
	{
		skill_t* s = game.player->skills;
		for (size_t i = 0; i < game.u->n_skills; i++)
			s[i] = 1000;
	}

	game_loop(&game);
	game_exit(&game);
	return 0;
}
