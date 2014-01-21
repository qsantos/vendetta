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

#include "menu.h"

#include <stdlib.h>
#include <SFML/Graphics.h>

void menu(settings_t* s, graphics_t* gr)
{
	(void) s;

	int id = graphics_spriteForImg(gr, "menu.png");
	sfSprite* illustration = gr->sprites[id];

	sfRenderWindow* render = gr->render;
	while (sfRenderWindow_isOpen(render))
	{
		sfEvent event;
		while (sfRenderWindow_pollEvent(render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				graphics_exit(gr);
				exit(0);
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView* view = sfView_createFromRect(rect);
				sfRenderWindow_setView(render, view);
				sfView_destroy(view);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				if (event.key.code == sfKeyEscape)
				{
					graphics_exit(gr);
					exit(0);
				}
				else if (event.key.code == sfKeyReturn)
					return;
			}
		}

		sfRenderWindow_clear(render, sfBlack);

		sfVector2u size  = sfRenderWindow_getSize(render);
		sfFloatRect rect = sfSprite_getGlobalBounds(illustration);

		sfVector2f pos = {size.x/2 - rect.width/2, size.y/2-rect.height/2};
		sfSprite_setPosition(illustration, pos);
		sfRenderWindow_drawSprite(render, illustration, NULL);

		graphics_drawCursor(gr, 0);
		sfRenderWindow_display(render);
	}
}
