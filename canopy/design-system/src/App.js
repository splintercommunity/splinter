/**
 * Copyright 2019 Cargill Incorporated
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import React, { lazy, Suspense } from 'react';
import {
  BrowserRouter as Router,
  Route,
  Switch,
  Redirect
} from 'react-router-dom';

import './App.scss';

import SideNav from './components/navigation/SideNav';
import Components from './views/Components';

const tabs = [
  {
    name: 'Overview',
    nested: [
      {
        name: 'Introduction',
        route: '/overview/introduction'
      },
      {
        name: 'Conventions',
        route: '/overview/conventions'
      }
    ]
  },
  {
    name: 'Design',
    nested: [
      {
        name: 'Colors',
        route: '/design/colors'
      },
      {
        name: 'Buttons',
        route: '/design/buttons'
      },
      {
        name: 'Typography',
        route: '/design/typography'
      }
    ]
  },
  {
    name: 'Components',
    nested: [
      {
        name: 'Modals',
        route: '/design/modals'
      }
    ]
  }
];

const Introduction = lazy(() =>
  import('!babel-loader!mdx-loader!./views/Introduction.mdx')
);
const Colors = lazy(() =>
  import('!babel-loader!mdx-loader!./views/Colors.mdx')
);
const Typography = lazy(() =>
  import('!babel-loader!mdx-loader!./views/Typography.mdx')
);

function App() {
  return (
    <Router>
      <div className="App">
        <SideNav tabs={tabs} />
        <div className="view marginLeft-l marginRight-l paddingTop-l">
          <Switch>
            <Redirect exact from="/" to="/overview/introduction" />
            <Redirect exact from="/overview" to="/overview/introduction" />
            <Route path="/overview/introduction">
              <Suspense fallback={<div>Loading...</div>}>
                <Introduction />
              </Suspense>
            </Route>
            <Redirect exact from="/design" to="/design/colors" />
            <Route path="/design/colors">
              <Suspense fallback={<div>Loading...</div>}>
                <Colors />
              </Suspense>
            </Route>
            <Route path="/design/typography">
              <Suspense fallback={<div>Loading...</div>}>
                <Typography />
              </Suspense>
            </Route>
            <Route path="/components">
              <Components />
            </Route>
          </Switch>
        </div>
      </div>
    </Router>
  );
}

export default App;
