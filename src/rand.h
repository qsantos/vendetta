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

#ifndef RAND_H
#define RAND_H

#include <limits.h>
#include <stdlib.h>

static inline float frnd(float min, float max)
{
	return min + ((float) rand() / RAND_MAX) * (max-min);
}
static inline float cfrnd(float max)
{
	return frnd(-max/2, max/2);
}
static inline int rnd_pick(const float* probas)
{
	float selected = frnd(0, 1);
	int i = 0;
	while (selected > probas[i])
	{
		selected -= probas[i];
		i++;
	}
	return i;
}

#endif
