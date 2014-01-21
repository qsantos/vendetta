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

#ifndef O_BUILD_H
#define O_BUILD_H

typedef struct ov_build ov_build_t;

#include "../universe/building.h"

struct ov_build
{
	kindOf_building_t* selected;

	char retracted;

	// quick access
	char  active;
	float x;
	float y;
	float radius;
	int   count;
	int*  list;
};

#include "../game.h"

void ov_build_init(ov_build_t* o);
void ov_build_exit(ov_build_t* o);

int  ov_build_draw  (ov_build_t* o, game_t* g, char do_draw);
int  ov_build_cursor(ov_build_t* o, game_t* g);
char ov_build_catch (ov_build_t* o, game_t* g, int t);

#endif
