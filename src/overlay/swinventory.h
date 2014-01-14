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

#ifndef O_SW_INVENTORY_H
#define O_SW_INVENTORY_H

typedef struct swinventory swinventory_t;

#include "subwindow.h"

struct swinventory
{
	subwindow_t w;
};

#include "../game.h"

void swinventory_init(swinventory_t* w, graphics_t* g);
void swinventory_exit(swinventory_t* w);

int  swinventory_draw  (swinventory_t* w, game_t* g, char do_draw);
char swinventory_cursor(swinventory_t* w, game_t* g, int x, int y);
char swinventory_catch (swinventory_t* w, game_t* g, int x, int y, int t);

#endif
