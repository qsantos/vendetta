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

#ifndef U_EVENT_H
#define U_EVENT_H

typedef struct kindOf_event kindOf_event_t;

#include <SFML/Audio.h>

#include "../graphics.h"

struct kindOf_event
{
	// graphics
	int sprite;
	int steps;
	float duration;
	float width;
	float height;

	// sound
	sfSound* sound; // TODO
};

void kindOf_event_init(kindOf_event_t* e);
void kindOf_event_exit(kindOf_event_t* e);

void kindOf_event_sprite(kindOf_event_t* e, graphics_t* g, const char* filename, int steps);
void kindOf_event_sound (kindOf_event_t* e, const char* filename);

#endif
