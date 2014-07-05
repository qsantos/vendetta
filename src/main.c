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

#include "version.h"
#include "menu.h"

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
		"  -v, --verbose N   sets the verbosity level:\n"
		"                    0 is quiet\n"
		"                    1 is normal\n"
		"                    2 is verbose\n"
		"                    3 is debug\n"
		"  -s, --size W H    set map's size to WxH tiles (>20)\n"
		"  -b, --bots N      set the number of bots\n"
		"  -r, --seed seed   set generation seed\n"
		"                    if this parameter is omitted, the seed\n"
		"                    is generated from the current time\n"
		"  -q, --quickstart  give player 20 of each material\n"
		"  -g, --godmode     give player god-like skills\n"
		, name
	);
	exit(1);
}

#ifdef __WIN32__
#undef _WIN32_WINNT
#define _WIN32_WINNT 0x0501
#include <windows.h>
int APIENTRY WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow)
{
	(void) hInstance;
	(void) hPrevInstance;
	(void) lpCmdLine;
	(void) nCmdShow;
	if (AttachConsole(ATTACH_PARENT_PROCESS))
	{
		freopen("CONOUT$","wb",stdout);
		freopen("CONOUT$","wb",stderr);
	}
	int argc;
	LPWSTR* argvw = CommandLineToArgvW(GetCommandLineW(), &argc);
	char** argv = malloc(sizeof(char)*(argc+1));
	for (int i = 0; i < argc; i++)
	{
		size_t n =
		WideCharToMultiByte(CP_UTF8, 0, argvw[i], -1,    NULL, 0, NULL, NULL);argv[i] = malloc(n);
		WideCharToMultiByte(CP_UTF8, 0, argvw[i], -1, argv[i], n, NULL, NULL);
	}
	argv[argc] = NULL;
	LocalFree(argvw);
#else
int main(int argc, char** argv)
{
#endif
	setlocale(LC_ALL, "");
	setlocale(LC_NUMERIC, "C");

	settings_t s =
	{
		.seed       = time(NULL),
		.map_width  = 500,
		.map_height = 500,
		.bots_count = 100,
		.verbosity  = 1,
		.godmode    = 0,
		.quickstart = 0,
	};

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
			fprintf(stderr, "Vendetta version " VERSION "\n");
			fprintf(stderr, "Compiled on %s at %s\n", __DATE__, __TIME__);
			exit(1);
		}
		else if (strcmp(option, "--verbose") == 0 || strcmp(option, "-v") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "Missing verbosity level\n");
				usage(argv[0]);
			}
			int level = strtol(argv[curarg++], NULL, 0);
			if (!(0 <= level && level <= 3))
			{
				fprintf(stderr, "Verbosity level must be between 0 and 3\n");
				usage(argv[0]);
			}
			s.verbosity = level;
		}
		else if (strcmp(option, "--size") == 0 || strcmp(option, "-s") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "No width given\n");
				usage(argv[0]);
			}
			int width = strtol(argv[curarg++], NULL, 0);
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
			int height = strtol(argv[curarg++], NULL, 0);
			if (height < MAP_MIN_HEIGHT)
			{
				fprintf(stderr, "Height must be at least %i (%i given)\n", MAP_MIN_HEIGHT, height);
				usage(argv[0]);
			}
			s.map_width  = width;
			s.map_height = height;
		}
		else if (strcmp(option, "--bots") == 0 || strcmp(option, "-b") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "No number was given\n");
				usage(argv[0]);
			}
			int n_bots = strtol(argv[curarg++], NULL, 0);
			if (n_bots < 0)
			{
				fprintf(stderr, "Must be a positive number (%i given)\n", n_bots);
				usage(argv[0]);
			}
			s.bots_count = n_bots;
		}
		else if (strcmp(option, "--seed") == 0 || strcmp(option, "-r") == 0)
		{
			if (curarg == argc)
			{
				fprintf(stderr, "No seed was given\n");
				usage(argv[0]);
			}
			s.seed = strtol(argv[curarg++], NULL, 0);
		}
		else if (strcmp(option, "--quickstart") == 0 || strcmp(option, "-q") == 0)
		{
			s.quickstart = 1;
		}
		else if (strcmp(option, "--godmode") == 0 || strcmp(option, "-g") == 0)
		{
			s.godmode = 1;
		}
		else
		{
			fprintf(stderr, "Unknown option '%s'\n", option);
			usage(argv[0]);
		}
	}
#ifdef __WIN32__
	for (int i = 0; i < argc; i++)
		free(argv[i]);
	free(argv);
#endif

	menu(&s);

	return 0;
}
